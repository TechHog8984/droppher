use std::{env, fs::{read_dir, File}, io::Read, process::exit, thread::sleep, time::{Duration, SystemTime}};

use regex::Regex;
use messages::prelude::*;
use notify_rust::Notification;
use json::{self, JsonValue};
use reqwest;

#[cfg(not(windows))]
use dialog::DialogBox;

#[cfg(windows)]
use win_dialog::{style, Icon, WinDialog};

enum FailType {
    Cancel,
    FailedToFindLogDirectory,
    FailedToFetchMapInformation
}

enum YesNoCancel {
    Yes,
    No,
    Cancel
}

enum SupportedLanguage {
    Spanish,
    English
}

struct LineHandler {
    selected_maps: Vec<String>,
    game_start_first_map_delay: Duration,

    map_finish_text: &'static str,
    selected_maps_pattern: Regex,
    map_information: JsonValue
}

#[async_trait]
impl Actor for LineHandler {}

#[async_trait]
impl Notifiable<String> for LineHandler {
    async fn notify(&mut self, input: String, _context: &Context<Self>) {
        if input.starts_with(self.map_finish_text) {
            if self.selected_maps.len() > 0 {
                handle_map(self.selected_maps.remove(0), &self.map_information);
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
            handle_map(self.selected_maps.remove(0), &self.map_information);

            return;
        };
    }
}

fn handle_map(map: String, map_information: &JsonValue) {
    let map_info = &map_information[map.as_str()];

    let map_info_str = if *map_info == JsonValue::Null {"".to_string()} else {
        format!("{} | {}{}{}",
            map_info["difficulty"].as_str().unwrap(),
            map_info["tip"].as_str().or(Some("no tip")).unwrap().to_string(),
            if map_info["portal_skip"] == JsonValue::Boolean(true) {
                " | SKIP"
            } else {
                ""
            },
            if map_info["portal_skip_tip"] != JsonValue::Null {
                format!(" ({})", map_info["portal_skip_tip"].as_str().unwrap())
            } else {
                "".to_string()
            }
        )
    };

    Notification::new()
        .summary("Droppher")
        .body(format!("You are now on {}\n{}", map, map_info_str).as_str())
        .timeout(5000)
        .show().expect("Failed to show notification");
}

#[cfg(windows)]
fn yes_no_dialog(title: &str, body: &str) -> YesNoCancel {
    match WinDialog::new(body)
        // TODO: header does not work (in vm testing at least)
        .with_header(title)
        .with_style(style::YesNoCancel)
        .with_icon(Icon::Information)
        .show()
        .unwrap_or_else(|_| {style::YesNoCancelResponse::Cancel})
    {
        style::YesNoCancelResponse::Yes => {
            YesNoCancel::Yes
        },
        style::YesNoCancelResponse::No => {
            YesNoCancel::No
        },
        style::YesNoCancelResponse::Cancel => {
            YesNoCancel::Cancel
        }
    }
}

#[cfg(not(windows))]
fn yes_no_dialog(title: &str, body: &str) -> YesNoCancel {
    match dialog::Question::new(body)
        .title(title)
        .show()
        .unwrap_or_else(|_| dialog::Choice::Cancel)
    {
        dialog::Choice::Yes => {
            YesNoCancel::Yes
        },
        dialog::Choice::No => {
            YesNoCancel::No
        },
        dialog::Choice::Cancel => {
            YesNoCancel::Cancel
        }
    }
}

#[tokio::main]
async fn main() {
    Notification::new()
        .summary("Droppher")
        .body("Droppher has started!")
        .show().expect("Failed to display notification");

    let log_path_option = match yes_no_dialog("Droppher - Which Client?", "Are you running Lunar (yes) or Badlion (no)?") {
        YesNoCancel::Yes => get_lunar_client_log_path(),
        YesNoCancel::No => get_badlion_log_path(),
        YesNoCancel::Cancel => {
            fail(FailType::Cancel);
            exit(0);
        }
    };

    let log_path = log_path_option.unwrap_or_else(|| {
        fail(FailType::FailedToFindLogDirectory);
        exit(0);
    });

    let language = match yes_no_dialog("Droppher - What Language?", "English (yes) or Español (no)?") {
        YesNoCancel::Yes => SupportedLanguage::English,
        YesNoCancel::No => SupportedLanguage::Spanish,
        YesNoCancel::Cancel => {
            fail(FailType::Cancel);
            exit(0);
        }
    };

    Notification::new()
        .summary("Droppher")
        .body(format!("Running with log: {}", log_path).as_str())
        .show().expect("Failed to display notification");

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

        map_finish_text: match language {
            SupportedLanguage::Spanish => "¡Terminaste el Mapa ",
            SupportedLanguage::English => "You finished Map "
        },
        selected_maps_pattern: Regex::new(match language {
            SupportedLanguage::Spanish => r"Mapas Seleccionados: ([\w', ]+)",
            SupportedLanguage::English => r"Selected Maps: ([\w', ]+)"
        }).unwrap(),
        map_information: json::parse(json_text.unwrap().as_str()).unwrap()
    }.spawn();

    let mut log_file: File;
    let mut buf: String = String::new();
    let mut last: String = String::new();

    log_file = File::open(&log_path).expect("Failed to open log file");
    log_file.read_to_string(&mut buf).expect("Failed to read log file");
    
    let mut line_index = buf.lines().count() - 1;

    loop {
        let old_last = last;
        log_file = File::open(&log_path).unwrap();

        buf.clear();
        log_file.read_to_string(&mut buf).expect("Failed to read log file");
    
        let lines = buf.lines().collect::<Vec<_>>();
        if lines.len() <= line_index {
            last = old_last;
            continue;
        }

        last = lines[line_index].to_string();
        line_index += 1;

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