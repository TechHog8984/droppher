use std::env;
use std::io::Read;
use std::process::exit;
// use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::time::{Duration, SystemTime};

use eframe::egui;
use regex::Regex;
use notify_rust::Notification;
use json::{self, JsonValue};

#[cfg(not(windows))]
use tinyfiledialogs;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 420.0])
            .with_resizable(false)
            .with_always_on_top()
            .with_transparent(true),
        ..Default::default()
    };

    let json_resp = reqwest::get("https://raw.githubusercontent.com/TechHog8984/droppher/master/assets/map_information.json")
        .await;

    if json_resp.is_err() {
        exit(0);
    }

    let json_text = json_resp.unwrap().text().await;
    if json_text.is_err() {
        exit(0);
    }

    eframe::run_native(
        "Droppher",
        options,
        Box::new(|_cc| {
            // let mut bedwars_player_infos = Vec::new();

            // bedwars_player_infos.push(BedwarsPlayerInfo{
            //     name: "Hello".to_string(),
            //     fkdr: 100.0,
            //     level: 1600.0
            // });

            let map_information = json::parse(json_text.unwrap().as_str()).unwrap();
            // let mut map_tip_map = HashMap::new();

            // for (name, member) in map_information.entries() {
            //     map_tip_map.insert(name.to_string(), member["tip"].as_str().unwrap_or("none").to_string());
            // }

            Box::<DroppherApp>::new(DroppherApp{
                page: MainPage::Global,
                enabled: false,
    
                hypixel_language: SupportedLanguage::English,
                client: SupportedClient::None,

                color_code_pattern: Regex::new("§.").unwrap(),
                chat_pattern: Regex::new(r"\[CHAT\] (.+)").unwrap(),
                player_chat1_pattern: Regex::new(r"^[\w\d_]+: ").unwrap(),
                player_chat2_pattern: Regex::new(r"^\[[\w+]+\] [\w\d_]+: ").unwrap(),

                map_finish_text: "".to_string(),
                selected_maps_pattern: Regex::new("").unwrap(),

                log_path: "".to_string(),
                last_line: "".to_string(),
                log_buffer: "".to_string(),
                last_line_index: 0,

                selected_maps: Vec::new(),
                // game_start_first_map_delay: Duration::from_secs(3),

                map_information,
                // map_tip_map,
    
                // bedwars_player_infos
            })
        })
    )
}

enum MainPage {
    Bedwars,
    Dropper,
    Global
}

#[derive(PartialEq)]
enum SupportedLanguage {
    English,
    Spanish
}

#[derive(PartialEq)]
#[derive(Clone)]
enum SupportedClient {
    None,
    Lunar,
    Badlion,
    #[cfg(not(windows))]
    Custom(String)
}

// struct BedwarsPlayerInfo {
//     name: String,
//     fkdr: f32,
//     level: f32
// }

struct DroppherApp {
    page: MainPage,
    enabled: bool,

    hypixel_language: SupportedLanguage,
    client: SupportedClient,

    color_code_pattern: Regex,
    chat_pattern: Regex,
    player_chat1_pattern: Regex,
    player_chat2_pattern: Regex,

    map_finish_text: String,
    selected_maps_pattern: Regex,

    log_path: String,
    last_line: String,
    log_buffer: String,
    last_line_index: usize,

    selected_maps: Vec<String>,
    // game_start_first_map_delay: Duration,

    map_information: JsonValue,
    // map_tip_map: HashMap<String, String>,

    // bedwars_player_infos: Vec<BedwarsPlayerInfo>
}

impl DroppherApp {
    fn read_log(&mut self) {
        let old_last = self.last_line.clone();
        let mut log_file = File::open(&self.log_path).unwrap();

        self.log_buffer.clear();
        log_file.read_to_string(&mut self.log_buffer).expect("Failed to read log file");

        let lines = self.log_buffer.lines().collect::<Vec<_>>();
        if lines.len() <= self.last_line_index {
            return;
        }
        if self.last_line_index == 0 {
            self.last_line_index = lines.len() - 1;
        }

        self.last_line = lines[self.last_line_index].to_string();
        self.last_line_index += 1;

        if !self.last_line.eq(&old_last) && self.last_line.split("[CHAT] ").count() == 2 {
            // save unmodified line so the old check still is valid
            let old_last = self.last_line.clone();

            self.last_line = self.color_code_pattern.replace_all(&self.last_line.as_str(), "").to_string();

            let result = self.chat_pattern
                .captures(&self.last_line.as_str());

            if result.is_some() {
                self.last_line = result.unwrap().get(1).unwrap().as_str().to_string();

                let is_player_chat = 
                    (self.player_chat1_pattern.captures(&self.last_line.as_str())).is_some() ||
                    (self.player_chat2_pattern.captures(&self.last_line.as_str())).is_some();

                if !is_player_chat {
                    let input = self.last_line.clone();
                    if input.starts_with(&self.map_finish_text) {
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
            
                        if Notification::new()
                            .summary("Droppher")
                            .body(format!("Game started! Maps: {:?}", self.selected_maps.clone()).as_str())
                            .timeout(5000)
                            .show().is_err()
                        {
                            println!("failed to display notification");
                        }
            
                        // sleep(self.game_start_first_map_delay);
                        handle_map(self.selected_maps.remove(0), &self.map_information);
            
                        return;
                    };
                }
            }

            self.last_line = old_last;
        }
    }
}

impl eframe::App for DroppherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.enabled {
            self.read_log();
        }

        egui::CentralPanel::default().frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT)).show(ctx, |ui| {
            ui.heading("Droppher by techhog");
            ui.hyperlink("https://github.com/TechHog8984/droppher");

            if ui.button(match self.enabled {false => "Start", true => "Stop"}).clicked() {
                let new_state = !self.enabled;
                if new_state {
                    if self.log_path.is_empty() {
                        if Notification::new()
                            .summary("Droppher")
                            .body("Select a client in Global before starting")
                            .show().is_err()
                        {
                            println!("failed to display notification");
                        }
                        return;
                    };

                    self.last_line_index = 0;
                    self.last_line.clear();

                    if Notification::new()
                        .summary("Droppher")
                        .body("Droppher has started!")
                        .show().is_err()
                    {
                        println!("failed to display notification");
                    }
                } else {
                    if Notification::new()
                        .summary("Droppher")
                        .body("Droppher has stopped!")
                        .show().is_err()
                    {
                        println!("failed to display notification");
                    }
                }
                self.enabled = new_state;
            };

            ui.horizontal(|ui| {
                if ui.button(format!("{}Bedwars", match self.page {MainPage::Bedwars => "*", _ => ""})).clicked() {
                    self.page = MainPage::Bedwars;
                } else if ui.button(format!("{}Dropper", match self.page {MainPage::Dropper => "*", _ => ""})).clicked() {
                    self.page = MainPage::Dropper;
                } else if ui.button(format!("{}Global", match self.page {MainPage::Global => "*", _ => ""})).clicked() {
                    self.page = MainPage::Global;
                }
            });

            match self.page {
                MainPage::Bedwars => {
                    ui.label("Work In Progress...");
                    // for info in &self.bedwars_player_infos {
                    //     ui.label(format!("{} | fkdr: {} | level: {}", info.name, info.fkdr, info.level));
                    // }
                },

                MainPage::Dropper => {
                    ui.label("Work In Progress...");
                    // TODO: the only real thing that this needs (besides the obvious missing members) is reading from and saving to a config file
                    // ui.collapsing("Maps", |ui| {
                    //     egui::ScrollArea::vertical().show(ui, |ui| {
                    //         for (name, member) in self.map_information.entries() {
                    //             ui.label(name);
                    //             ui.indent(1, |ui| {
                    //                 ui.label(format!("difficulty: {}", member["difficulty"].as_str().unwrap()));
                                    
                    //                 ui.horizontal(|ui| {
                    //                     ui.label("tip: ");
    
                    //                     let mut tip = self.map_tip_map.get(&name.to_string()).unwrap().to_string();
                    //                     ui.text_edit_singleline(&mut tip);
                    //                     self.map_tip_map.insert(name.to_string(), tip);
                    //                 });
                    //             });
                    //         }
                    //     });
                    // });
                },

                MainPage::Global => {
                    ui.label("Hypixel language:");
                    ui.horizontal(|ui| {
                        if ui.radio_value(&mut self.hypixel_language, SupportedLanguage::English, "English").changed() ||
                            ui.radio_value(&mut self.hypixel_language, SupportedLanguage::Spanish, "Español").changed() 
                        {
                            self.map_finish_text = match self.hypixel_language {
                                SupportedLanguage::Spanish => "¡Terminaste el Mapa ".to_string(),
                                SupportedLanguage::English => "You finished Map ".to_string()
                            };

                            self.selected_maps_pattern = Regex::new(match self.hypixel_language {
                                SupportedLanguage::Spanish => r"Mapas Seleccionados: ([\w', ]+)",
                                SupportedLanguage::English => r"Selected Maps: ([\w', ]+)"
                            }).unwrap();
                        };
                    });

                    ui.label("Client:");
                    ui.horizontal(|ui| {
                        let old_client = self.client.clone();

                        let lunar_radio = ui.radio_value(&mut self.client, SupportedClient::Lunar, "Lunar");
                        let badlion_radio = ui.radio_value(&mut self.client, SupportedClient::Badlion, "Badlion");

                        #[allow(unused_mut)]
                        let mut custom_file_changed = false;

                        #[cfg(not(windows))]
                        if ui.button("Select Log Directory").clicked() {
                            custom_file_changed = true;
                            match tinyfiledialogs::select_folder_dialog("Select Log Directory", "") {
                                Some(result) => {self.client = SupportedClient::Custom(result)},
                                None => {self.client = SupportedClient::Custom("".to_string())}
                            };
                        };

                        if lunar_radio.changed() || badlion_radio.changed() || custom_file_changed {
                            let path_option = match &self.client {
                                SupportedClient::None => None,
                                SupportedClient::Lunar => verify_path(&get_lunar_client_log_path()),
                                SupportedClient::Badlion => verify_path(&get_badlion_log_path()),
                                #[cfg(not(windows))]
                                SupportedClient::Custom(path) => verify_path(&get_latest_file_path(path.to_string()))
                            };

                            match path_option {
                                Some(path) => {self.log_path = path;},
                                None => {
                                    self.client = old_client;

                                    if Notification::new()
                                        .summary("Droppher")
                                        .body("Failed to find log path")
                                        .show().is_err()
                                    {
                                        println!("failed to display notification");
                                    }
                                }
                            };
                        };
                    });

                    ui.label(format!("Log path: {}", self.log_path));
                }
            };
        });

        ctx.request_repaint();
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

    if Notification::new()
        .summary("Droppher")
        .body(format!("You are now on {}\n{}", map, map_info_str).as_str())
        .timeout(5000)
        .show().is_err()
    {
        println!("failed to display notification");
    }
}

fn verify_path(path: &Option<String>) -> Option<String> {
    match path {
        Some(path) => {
            let file = File::open(path);

            if file.is_ok() {
                Some(path.to_string())
            } else {
                None
            }
        },
        None => None
    }
}

fn get_latest_file_path(directory_path: String) -> Option<String> {
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

    get_latest_file_path(directory_path)
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