use std::{env, fs::{read_dir, File}, io::Read, process::exit, thread::sleep, time::{Duration, SystemTime}};

use regex::Regex;
use messages::prelude::*;
use notify_rust::Notification;
use dialog::DialogBox;

enum FailType {
    Cancel,
    FailedToFindLogDirectory
}

struct LineHandler {
    selected_maps: Vec<String>,

    selected_maps_pattern: Regex
}

#[async_trait]
impl Actor for LineHandler {}

#[async_trait]
impl Notifiable<String> for LineHandler {
    async fn notify(&mut self, input: String, _context: &Context<Self>) {
        if input.eq("¡SALTA!") || input.starts_with("¡Terminaste el Mapa ") {
            handle_map(self.selected_maps.remove(0)).await;
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

            return;
        };
    }
}

async fn handle_map(map: String) {
    Notification::new()
        .summary("Droppher")
        .body(format!("You are now on {}", map).as_str())
        .show().expect("Failed to show notification");
}

#[tokio::main]
async fn main() {
    Notification::new()
        .summary("Droppher")
        .body("Droppher has started!")
        .show().expect("Failed to display notification");

    let client_option = dialog::Question::new("Are you running Lunar (yes) or Badlion (no)?")
        .title("Client")
        .show()
        .unwrap_or_else(|_| {dialog::Choice::Cancel});

    let log_path_option: Option<String>;

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

    let mut handler = LineHandler{
        selected_maps: Vec::new(),

        selected_maps_pattern: Regex::new(r"Mapas Seleccionados: ([\w', ]+)").unwrap(),
    }.spawn();

    loop {
        let old_last = last;
        log_file = File::open(log_path.clone()).unwrap();

        buf.clear();
        log_file.read_to_string(&mut buf).expect("Failed to read log file");
    
        last = buf.lines().last().unwrap().to_string();

        if !last.eq(&old_last) && last.split("[CHAT] ").count() == 2 {
            // save unmodified line to the old check still is valid
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
            FailType::FailedToFindLogDirectory => "Failed to find log directory"
        })
        .show().expect("Failed to display notification");
}

fn get_lunar_client_log_path() -> Option<String> {
    let user_path_result = env::var("USER");

    let directory_path = match user_path_result {
        Ok(path) => format!("/home/{}/.lunarclient/logs/game", path),
        Err(_) => return None
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

fn get_badlion_log_path() -> Option<String> {

    // TODO: Badlion support
    None
}