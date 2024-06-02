use std::{env, fs::{read_dir, File}, io::Read, process::exit, thread::sleep, time::{Duration, SystemTime}};

use regex::Regex;
use messages::prelude::*;
use notify_rust::Notification;
use dialog::DialogBox;
use json::{self, JsonValue};
use reqwest;

#[cfg(windows)]
use win_dialog::{style, Icon, WinDialog};

enum FailType {
    Cancel,
    FailedToFindLogDirectory,
    FailedToFetchMapInformation
}

struct LineHandler {
    selected_maps: Vec<String>,
    game_start_first_map_delay: Duration,

    selected_maps_pattern: Regex,
    map_information: JsonValue
}

#[async_trait]
impl Actor for LineHandler {}

#[async_trait]
impl Notifiable<String> for LineHandler {
    async fn notify(&mut self, input: String, _context: &Context<Self>) {
        if input.starts_with("¡Terminaste el Mapa ") {
            if self.selected_maps.len() > 0 {
                handle_map(self.selected_maps.remove(0), &self.map_information).await;
            }
            return;
        }

        let result = self.selected_maps_pattern
            .captures(input.as_str());

        if result.is_some() {
            self.selected_maps.clear();

            let captures = result.unwrap();
            for map in captures.get(1).unwrap().as_str().split(", ") {
                self.selected_maps.push(map.to_string());
            };

            Notification::new()
                .summary("Droppher")
                .body(format!("Game started! Maps: {:?}", self.selected_maps.clone()).as_str())
                .timeout(5000)
                .show().expect("Failed to show notification");

            sleep(self.game_start_first_map_delay);
            handle_map(self.selected_maps.remove(0), &self.map_information).await;

            return;
        };
    }
}

async fn handle_map(map: String, map_information: &JsonValue) {
    let map_info = &map_information[map.as_str()];

    let map_info_str = if *map_info == JsonValue::Null {"".to_string()} else {
        format!("{} | {}", map_info["difficulty"].as_str().unwrap(), map_info["tip"].as_str().or(Some("no tip")).unwrap().to_string())
    };

    Notification::new()
        .summary("Droppher")
        .body(format!("You are now on {}\n{}", map, map_info_str).as_str())
        .timeout(5000)
        .show().expect("Failed to show notification");
}

#[cfg(windows)]
fn windows_client_dialog(log_path_option: &mut Option<String>) {
    let client_option = WinDialog::new("Are you running Lunar (yes) or Badlion (no)?")
        // TODO: header does not work (in vm testing at least)
        .with_header("Droppher - Which Client?")
        .with_style(style::YesNoCancel)
        .with_icon(Icon::Information)
        .show()
        .unwrap_or_else(|_| {style::YesNoCancelResponse::Cancel});

    match client_option {
        style::YesNoCancelResponse::Yes => {
            *log_path_option = get_lunar_client_log_path();
        },
        style::YesNoCancelResponse::No => {
            *log_path_option = get_badlion_log_path();
        },
        style::YesNoCancelResponse::Cancel => {
            fail(FailType::Cancel);
            exit(0);
        }
    };
}

#[tokio::main]
async fn main() {
    Notification::new()
        .summary("Droppher")
        .body("Droppher has started!")
        .show().expect("Failed to display notification");

    let mut log_path_option: Option<String> = None;

    if cfg!(windows) {
        #[cfg(windows)]
        windows_client_dialog(&mut log_path_option);
    } else {
        let client_option = dialog::Question::new("Are you running Lunar (yes) or Badlion (no)?")
            .title("Droppher - Which Client?")
            .show()
            .unwrap_or_else(|_| {dialog::Choice::Cancel});

        match client_option {
            dialog::Choice::Yes => {
                log_path_option = get_lunar_client_log_path();
            },
            dialog::Choice::No => {
                log_path_option = get_badlion_log_path();
            },
            dialog::Choice::Cancel => {
                fail(FailType::Cancel);
                exit(0);
            }
        };
    }

    let log_path = log_path_option.unwrap_or_else(|| {
        fail(FailType::FailedToFindLogDirectory);
        exit(0);
    });

    Notification::new()
        .summary("Droppher")
        .body(format!("Log file: {}", log_path).as_str())
        .show().expect("Failed to display notification");

    let mut log_file: File;
    let mut buf: String = String::new();
    let mut last: String = String::new();

    let delay = Duration::from_millis(100);

    let color_code_pattern = Regex::new("§.").unwrap();
    let chat_pattern = Regex::new(r"\[CHAT\] (.+)").unwrap();
    let player_chat1_pattern = Regex::new(r"^[\w\d_]+: ").unwrap();
    let player_chat2_pattern = Regex::new(r"^\[[\w+]+\] [\w\d_]+: ").unwrap();

    let json_resp = reqwest::get("https://raw.githubusercontent.com/TechHog8984/droppher/master/assets/map_information.json")
        .await;

    if json_resp.is_err() {
        fail(FailType::FailedToFetchMapInformation);
        exit(0);
    }

    let json_text = json_resp.unwrap().text().await;
    if json_text.is_err() {
        fail(FailType::FailedToFetchMapInformation);
        exit(0);
    }

    let mut handler = LineHandler{
        selected_maps: Vec::new(),
        game_start_first_map_delay: Duration::from_secs(3),

        selected_maps_pattern: Regex::new(r"Mapas Seleccionados: ([\w', ]+)").unwrap(),
        map_information: json::parse(json_text.unwrap().as_str()).unwrap()
    }.spawn();

    loop {
        let old_last = last;
        log_file = File::open(log_path.clone()).unwrap();

        buf.clear();
        log_file.read_to_string(&mut buf).expect("Failed to read log file");
    
        last = buf.lines().last().unwrap().to_string();

        if !last.eq(&old_last) && last.split("[CHAT] ").count() == 2 {
            // save unmodified line so the old check still is valid
            let old_last = last.clone();

            last = color_code_pattern.replace_all(last.as_str(), "").to_string();

            let result = chat_pattern
                .captures(last.as_str());

            if result.is_some() {
                last = result.unwrap().get(1).unwrap().as_str().to_string();

                let is_player_chat =
                    (player_chat1_pattern.captures(last.as_str())).is_some() ||
                    (player_chat2_pattern.captures(last.as_str())).is_some();
    
                if !is_player_chat {
                    handler.notify(last.clone()).await.unwrap();
                }
            }

            last = old_last;
        }

        sleep(delay);
    }
}

fn fail(fail_type: FailType) {
    Notification::new()
        .summary("Droppher")
        .body(match fail_type { 
            FailType::Cancel => "Canceled dialog",
            FailType::FailedToFindLogDirectory => "Failed to find log directory",
            FailType::FailedToFetchMapInformation => "Failed to fetch map information"
        })
        .show().expect("Failed to display notification");
}

#[cfg(not(windows))]
fn get_lunar_client_directory_path() -> Option<String> {
    match env::var("HOME") {
        Ok(path) => Some(format!("{}/.lunarclient/logs/game", path)),
        Err(_) => None
    }
}

#[cfg(windows)]
fn get_lunar_client_directory_path() -> Option<String> {
    match env::var("HOMEPATH") {
        Ok(homepath) => Some(format!("C:{}\\.lunarclient\\logs\\game", homepath)),
        Err(_) => None
    }
}

fn get_lunar_client_log_path() -> Option<String> {
    let directory_path = match get_lunar_client_directory_path() {
        Some(path) => path,
        None => return None
    };

    let entries_result = read_dir(directory_path);
    let entries = match entries_result {
        Ok(dir) => dir,
        Err(_) => return None
    };

    let time_now = SystemTime::now();
    let mut smallest_difference: Option<Duration> = None;
    let mut most_recently_modified_path: Option<String> = None;

    for file in entries {
        let file = match file {
            Ok(f) => f,
            Err(_) => return None
        };

        let metadata = match file.metadata() {
            Ok(m) => m,
            Err(_) => return None
        };

        let modified = match metadata.modified() {
            Ok(m) => m,
            Err(_) => return None
        };

        let difference = time_now.duration_since(modified)
            .expect("File was modified in the future");

        if smallest_difference.is_none() || difference < smallest_difference.unwrap() {
            smallest_difference.replace(difference);
            most_recently_modified_path = match file.path().to_str() {
                Some(str) => Some(str.to_string()),
                None => return None
            };
        }
    }

    most_recently_modified_path
}

#[cfg(not(windows))]
fn get_badlion_log_path() -> Option<String> {
    // TODO: non-windows badlion
    None
}

#[cfg(windows)]
fn get_badlion_log_path() -> Option<String> {
    match env::var("APPDATA") {
        Ok(appdata) => Some(format!("{}\\.minecraft\\logs\\blclient\\minecraft\\latest.log", appdata)),
        Err(_) => return None
    }
}