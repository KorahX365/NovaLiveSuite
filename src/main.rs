use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use axum::{
    extract::{State, ws::{WebSocketUpgrade, WebSocket, Message}},
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::{self, Stream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, CheckMenuItem, PredefinedMenuItem},
    TrayIconBuilder,
};
use winreg::enums::*;
use winreg::RegKey;

const PORT: u16 = 777;
const APP_REG_KEY: &str = "NovaLiveSuite";
const TWITCH_CLIENT_ID: &str = "fim46am8fzxb5foogfyk16pgybpkjt";
const TWITCH_CLIENT_SECRET: &str = "7vewrnzdw8y2gq78sag9a9hwce7ug9";
const TWITCH_REDIRECT_URI: &str = "http://localhost:777/twitch/callback";

fn default_true() -> bool { true }
fn default_theme() -> String { "glassmorphic".to_string() }
fn default_accent() -> String { "auto".to_string() }

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ChatSettings {
    font_family: String,
    font_size: String,
    color_user: String,
    color_text: String,
    bg_opacity: f32,
    bg_color: String,
    border_color: String,
    border_width: String,
    border_radius: String,
    fade_duration_s: u32,
    animation_in: String,
    max_messages: u32,
    message_spacing_px: u32,
    badge_size_px: u32,
    text_shadow: String,
    padding_px: u32,
    alignment: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct CarouselItem {
    id: String,
    #[serde(rename = "type")]
    item_type: String,
    url: String,
    duration_ms: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct CarouselSettings {
    interval_ms: u32,
    transition: String,
    items: Vec<CarouselItem>,
    width_px: String,
    height_px: String,
    border_radius: String,
    border_color: String,
    border_width: String,
    bg_color: String,
    bg_opacity: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct NowPlayingSettings {
    font_family: String,
    font_size: String,
    color_title: String,
    color_artist: String,
    color_bg: String,
    bg_opacity: f32,
    border_color: String,
    border_width: String,
    border_radius: String,
    scroll_text: bool,
    detect_spotify: bool,
    detect_chrome: bool,
    detect_firefox: bool,
    detect_system: bool,
    padding_px: u32,
    #[serde(default = "default_true")]
    vinyl_enabled: bool,
    #[serde(default = "default_true")]
    lyrics_enabled: bool,
    #[serde(default = "default_true")]
    visualizer_enabled: bool,
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default = "default_accent")]
    accent_color: String,
    #[serde(default)]
    hide_on_alert: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct AlertSettings {
    follow_duration_ms: u32,
    sub_duration_ms: u32,
    raid_duration_ms: u32,
    follow_template: String,
    sub_template: String,
    raid_template: String,
    follow_image: String,
    sub_image: String,
    raid_image: String,
    follow_sound: String,
    sub_sound: String,
    raid_sound: String,
    font_family: String,
    font_size: String,
    font_color: String,
    bg_color: String,
    bg_opacity: f32,
    border_color: String,
    border_width: String,
    border_radius: String,
    animation_in: String,
    animation_out: String,
    #[serde(default)]
    image_size: String,
    #[serde(default)]
    layout: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ToolChat {
    enabled: bool,
    settings: ChatSettings,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ToolCarousel {
    enabled: bool,
    settings: CarouselSettings,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ToolNowPlaying {
    enabled: bool,
    settings: NowPlayingSettings,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ToolAlerts {
    enabled: bool,
    settings: AlertSettings,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct PollSettings {
    font_family: String,
    font_size: String,
    bg_color: String,
    bg_opacity: f32,
    border_color: String,
    border_width: String,
    border_radius: String,
    padding_px: u32,
    default_duration_s: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ToolPolls {
    enabled: bool,
    settings: PollSettings,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct WorldCupSettings {
    font_family: String,
    font_size: String,
    bg_color: String,
    bg_opacity: f32,
    border_color: String,
    border_width: String,
    border_radius: String,
    team1_name: String,
    team1_score: u32,
    team2_name: String,
    team2_score: u32,
    match_time: String,
    timer_active: bool,
    logo_variant: String,
    accent_color: String,
    layout_mode: String,
    team1_code: String,
    team1_flag: String,
    team2_code: String,
    team2_flag: String,
    match_stage: String,
    scorer_name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ToolWorldCup {
    enabled: bool,
    settings: WorldCupSettings,
}

fn default_worldcup_tool() -> ToolWorldCup {
    ToolWorldCup {
        enabled: true,
        settings: WorldCupSettings {
            font_family: "Outfit".to_string(),
            font_size: "20px".to_string(),
            bg_color: "#0f0f16".to_string(),
            bg_opacity: 0.85,
            border_color: "#eab308".to_string(),
            border_width: "2px".to_string(),
            border_radius: "16px".to_string(),
            team1_name: "Spain".to_string(),
            team1_score: 0,
            team2_name: "United States".to_string(),
            team2_score: 0,
            match_time: "45:00".to_string(),
            timer_active: false,
            logo_variant: "horizontal_color".to_string(),
            accent_color: "#eab308".to_string(),
            layout_mode: "in_progress".to_string(),
            team1_code: "ESP".to_string(),
            team1_flag: "https://flagcdn.com/w80/es.png".to_string(),
            team2_code: "USA".to_string(),
            team2_flag: "https://flagcdn.com/w80/us.png".to_string(),
            match_stage: "GROUP STAGE - GROUP A".to_string(),
            scorer_name: "L. MESSI 10'".to_string(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Tools {
    chat: ToolChat,
    carousel: ToolCarousel,
    nowplaying: ToolNowPlaying,
    alerts: ToolAlerts,
    #[serde(default = "default_polls_tool")]
    polls: ToolPolls,
    #[serde(default = "default_worldcup_tool")]
    worldcup: ToolWorldCup,
}

fn default_polls_tool() -> ToolPolls {
    ToolPolls {
        enabled: true,
        settings: PollSettings {
            font_family: "Outfit".to_string(),
            font_size: "18px".to_string(),
            bg_color: "#0f0f16".to_string(),
            bg_opacity: 0.65,
            border_color: "#6366f1".to_string(),
            border_width: "1px".to_string(),
            border_radius: "20px".to_string(),
            padding_px: 24,
            default_duration_s: 60,
        }
    }
}

fn default_lang() -> String {
    "en".to_string()
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Config {
    twitch_channel: String,
    twitch_connected: bool,
    twitch_username: String,
    tools: Tools,
    #[serde(default = "default_lang")]
    lang: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            twitch_channel: "".to_string(),
            twitch_connected: false,
            twitch_username: "".to_string(),
            lang: "en".to_string(),
            tools: Tools {
                chat: ToolChat {
                    enabled: true,
                    settings: ChatSettings {
                        font_family: "Outfit".to_string(),
                        font_size: "18px".to_string(),
                        color_user: "#00f0ff".to_string(),
                        color_text: "#ffffff".to_string(),
                        bg_opacity: 0.4,
                        bg_color: "#0a0a0f".to_string(),
                        border_color: "#3b82f6".to_string(),
                        border_width: "1px".to_string(),
                        border_radius: "12px".to_string(),
                        fade_duration_s: 8,
                        animation_in: "slide-fade".to_string(),
                        max_messages: 10,
                        message_spacing_px: 10,
                        badge_size_px: 16,
                        text_shadow: "0 2px 4px rgba(0,0,0,0.5)".to_string(),
                        padding_px: 10,
                        alignment: "left".to_string(),
                    },
                },
                carousel: ToolCarousel {
                    enabled: true,
                    settings: CarouselSettings {
                        interval_ms: 5000,
                        transition: "fade".to_string(),
                        items: vec![
                            CarouselItem {
                                id: "1".to_string(),
                                item_type: "image".to_string(),
                                url: "https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?q=80&w=800".to_string(),
                                duration_ms: 5000,
                            },
                            CarouselItem {
                                id: "2".to_string(),
                                item_type: "image".to_string(),
                                url: "https://images.unsplash.com/photo-1600132806370-bf17e65e942f?q=80&w=800".to_string(),
                                duration_ms: 5000,
                            },
                        ],
                        width_px: "100%".to_string(),
                        height_px: "100%".to_string(),
                        border_radius: "0px".to_string(),
                        border_color: "#3b82f6".to_string(),
                        border_width: "0px".to_string(),
                        bg_color: "#000000".to_string(),
                        bg_opacity: 0.0,
                    },
                },
                nowplaying: ToolNowPlaying {
                    enabled: true,
                    settings: NowPlayingSettings {
                        font_family: "Outfit".to_string(),
                        font_size: "16px".to_string(),
                        color_title: "#00f0ff".to_string(),
                        color_artist: "#a78bfa".to_string(),
                        color_bg: "#0a0a0f".to_string(),
                        bg_opacity: 0.5,
                        border_color: "#3b82f6".to_string(),
                        border_width: "1px".to_string(),
                        border_radius: "10px".to_string(),
                        scroll_text: true,
                        detect_spotify: true,
                        detect_chrome: true,
                        detect_firefox: true,
                        detect_system: true,
                        padding_px: 12,
                        vinyl_enabled: true,
                        lyrics_enabled: true,
                        visualizer_enabled: true,
                        theme: "glassmorphic".to_string(),
                        accent_color: "auto".to_string(),
                        hide_on_alert: false,
                    },
                },
                alerts: ToolAlerts {
                    enabled: true,
                    settings: AlertSettings {
                        follow_duration_ms: 4000,
                        sub_duration_ms: 5000,
                        raid_duration_ms: 6000,
                        follow_template: "{user} is now following!".to_string(),
                        sub_template: "{user} subscribed!".to_string(),
                        raid_template: "{user} is raiding with {count} viewers!".to_string(),
                        follow_image: "https://media.giphy.com/media/v1.Y2lkPTc5MGI3NjExM3N2c2t3bnoxdmFidmx0eWd0Mm0wdXNldnR5czRpdWdta2I5dzdrZyZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9cw/L330J40l95cE9bU0W2/giphy.gif".to_string(),
                        sub_image: "https://media.giphy.com/media/v1.Y2lkPTc5MGI3NjExdm4yN2V6NWc3bW9yc2Z0Znp3ZnpvbmM1dTR4NXJ4MXR0Zzh5Zjl4ZyZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9cw/3orif2us7Oa9N2K9Ve/giphy.gif".to_string(),
                        raid_image: "https://media.giphy.com/media/134DVXcD94sOWI/giphy.gif".to_string(),
                        follow_sound: "".to_string(),
                        sub_sound: "".to_string(),
                        raid_sound: "".to_string(),
                        font_family: "Outfit".to_string(),
                        font_size: "24px".to_string(),
                        font_color: "#ffffff".to_string(),
                        bg_color: "#0a0a0f".to_string(),
                        bg_opacity: 0.6,
                        border_color: "#8b5cf6".to_string(),
                        border_width: "2px".to_string(),
                        border_radius: "16px".to_string(),
                        animation_in: "slide-fade".to_string(),
                        animation_out: "fade".to_string(),
                        image_size: "80px".to_string(),
                        layout: "top".to_string(),
                    },
                },
                polls: default_polls_tool(),
                worldcup: default_worldcup_tool(),
            },
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
struct TrackState {
    title: String,
    artist: String,
    album: String,
    playback_status: String,
    position: f64,
    duration: f64,
    source: String,
    thumbnail: String,
    source_app_id: String,
    lyrics: Vec<serde_json::Value>,
    year: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
struct TwitchTokens {
    access_token: String,
    refresh_token: String,
    user_id: String,
    user_login: String,
}

struct AppState {
    config: Mutex<Config>,
    sse_tx: broadcast::Sender<Value>,
    ws_tx: broadcast::Sender<String>,
    twitch_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    nowplaying_state: Mutex<TrackState>,
}

fn resolve_path(subpath: &str) -> PathBuf {
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop(); // pop exe file name
        
        let mut current = exe_path.clone();
        for _ in 0..5 {
            let path = current.join(subpath);
            if path.exists() {
                return path;
            }
            if !current.pop() {
                break;
            }
        }
    }
    
    if let Ok(cwd) = std::env::current_dir() {
        let path = cwd.join(subpath);
        if path.exists() {
            return path;
        }
    }
    
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        return exe_path.join(subpath);
    }
    
    PathBuf::from(subpath)
}

fn get_config_path() -> PathBuf {
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        return exe_path.join("config.json");
    }
    std::env::current_dir().unwrap_or_default().join("config.json")
}

fn load_config() -> Config {
    let path = get_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
                return cfg;
            }
        }
    }
    let default_cfg = Config::default();
    let _ = fs::write(&path, serde_json::to_string_pretty(&default_cfg).unwrap());
    default_cfg
}

fn save_config(cfg: &Config) {
    let path = get_config_path();
    let _ = fs::write(&path, serde_json::to_string_pretty(cfg).unwrap());
}

fn get_twitch_tokens_path() -> PathBuf {
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        return exe_path.join("twitch_tokens.json");
    }
    std::env::current_dir().unwrap_or_default().join("twitch_tokens.json")
}

fn load_twitch_tokens() -> TwitchTokens {
    let path = get_twitch_tokens_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(tokens) = serde_json::from_str::<TwitchTokens>(&content) {
                return tokens;
            }
        }
    }
    TwitchTokens::default()
}

fn save_twitch_tokens(tokens: &TwitchTokens) {
    let path = get_twitch_tokens_path();
    let _ = fs::write(&path, serde_json::to_string_pretty(tokens).unwrap());
}

fn is_autostart_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run") {
        key.get_value::<String, _>(APP_REG_KEY).is_ok()
    } else {
        false
    }
}

fn enable_autostart() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run") {
        if let Ok(exe_path) = std::env::current_exe() {
            let _ = key.set_value(APP_REG_KEY, &exe_path.to_string_lossy().to_string());
        }
    }
}

fn disable_autostart() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Run", KEY_WRITE) {
        let _ = key.delete_value(APP_REG_KEY);
    }
}

fn create_tray_icon_image() -> tray_icon::Icon {
    let width = 64;
    let height = 64;
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - 32.0;
            let dy = y as f32 - 32.0;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 12.0 {
                rgba.extend_from_slice(&[255, 255, 255, 255]);
            } else if dist < 22.0 {
                rgba.extend_from_slice(&[6, 182, 212, 255]);
            } else if dist < 28.0 {
                rgba.extend_from_slice(&[139, 92, 246, 255]);
            } else {
                rgba.extend_from_slice(&[8, 9, 12, 255]);
            }
        }
    }
    tray_icon::Icon::from_rgba(rgba, width as u32, height as u32).unwrap()
}

// Twitch Chat IRC Listener
fn start_twitch_chat(state: Arc<AppState>) {
    let mut handle_lock = state.twitch_handle.lock().unwrap();
    if let Some(h) = handle_lock.take() {
        h.abort();
    }

    let channel = state.config.lock().unwrap().twitch_channel.clone();
    let is_chat_enabled = state.config.lock().unwrap().tools.chat.enabled;

    if channel.is_empty() || !is_chat_enabled {
        return;
    }

    let sse_tx = state.sse_tx.clone();
    let new_handle = tokio::spawn(async move {
        let channel = channel.to_lowercase();
        let user = format!("justinfan{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() % 100000);
        
        loop {
            println!("Connecting to Twitch Chat IRC: #{}", channel);
            match TcpStream::connect("irc.chat.twitch.tv:6667").await {
                Ok(socket) => {
                    let (read_half, mut write_half) = tokio::io::split(socket);
                    let _ = write_half.write_all(b"PASS SCHMOOPIIE\r\n").await;
                    let _ = write_half.write_all(format!("NICK {}\r\n", user).as_bytes()).await;
                    let _ = write_half.write_all(b"CAP REQ :twitch.tv/tags\r\n").await;
                    let _ = write_half.write_all(format!("JOIN #{}\r\n", channel).as_bytes()).await;
                    
                    let reader = BufReader::new(read_half);
                    let mut lines = reader.lines();
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        if line.starts_with("PING") {
                            let _ = write_half.write_all(b"PONG :tmi.twitch.tv\r\n").await;
                        } else if line.contains("PRIVMSG") {
                            if let Some(msg_data) = parse_twitch_msg(&line) {
                                let _ = sse_tx.send(serde_json::json!({
                                    "event": "chat_message",
                                    "data": msg_data
                                }));
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Error connecting to twitch IRC: {}", e);
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    *handle_lock = Some(new_handle);
}

fn parse_twitch_msg(line: &str) -> Option<Value> {
    if !line.starts_with('@') {
        return None;
    }
    
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() < 3 {
        return None;
    }
    
    let tags_str = &parts[0][1..];
    let content_parts: Vec<&str> = parts[2].splitn(2, "PRIVMSG").collect();
    if content_parts.len() < 2 {
        return None;
    }
    
    let rest = content_parts[1].trim();
    let channel_and_msg: Vec<&str> = rest.splitn(2, " :").collect();
    if channel_and_msg.len() < 2 {
        return None;
    }
    
    let msg_text = channel_and_msg[1];
    
    let mut display_name = "".to_string();
    let mut color = "#FF69B4".to_string();
    let mut id = format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    let mut badges = Vec::new();
    
    for tag in tags_str.split(';') {
        let kv: Vec<&str> = tag.split('=').collect();
        if kv.len() == 2 {
            match kv[0] {
                "display-name" => display_name = kv[1].to_string(),
                "color" => {
                    if !kv[1].is_empty() {
                        color = kv[1].to_string();
                    }
                }
                "id" => id = kv[1].to_string(),
                "badges" => {
                    if !kv[1].is_empty() {
                        for badge in kv[1].split(',') {
                            let b_parts: Vec<&str> = badge.split('/').collect();
                            if !b_parts.is_empty() {
                                badges.push(b_parts[0].to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    if display_name.is_empty() {
        let prefix = parts[1];
        if let Some(idx) = prefix.find('!') {
            display_name = prefix[1..idx].to_string();
        }
    }
    
    Some(serde_json::json!({
        "user": display_name,
        "color": color,
        "message": msg_text,
        "badges": badges,
        "id": id
    }))
}

fn clean_metadata_for_search(text: &str) -> String {
    let mut cleaned = text.to_string();
    let re_remove = [
        r"(?i)[\(\[][Ff]eat\..*?[\)\]]",
        r"(?i)[\(\[][Ww]ith.*?[\)\]]",
        r"(?i)[\(\[][Rr]emaster.*?[\)\]]",
        r"(?i)[\(\[][Ll]ive.*?[\)\]]",
    ];
    for &pat in &re_remove {
        if let Ok(re) = regex::Regex::new(pat) {
            cleaned = re.replace_all(&cleaned, "").to_string();
        }
    }
    if let Some(idx) = cleaned.to_lowercase().find("feat.") {
        cleaned.truncate(idx);
    }
    if let Some(idx) = cleaned.to_lowercase().find("with ") {
        cleaned.truncate(idx);
    }
    
    cleaned = cleaned.chars().map(|c| {
        if c.is_alphanumeric() || c == ' ' || c == '-' || c == '\'' {
            c
        } else {
            ' '
        }
    }).collect();
    
    cleaned.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn get_readable_source(app_id: &str, title: &str, artist: &str, album: &str) -> String {
    let app_id_lower = app_id.to_lowercase();
    let title_lower = title.to_lowercase();
    let artist_lower = artist.to_lowercase();
    let album_lower = album.to_lowercase();
    
    if app_id_lower.contains("spotify") {
        return "Spotify".to_string();
    } else if app_id_lower.contains("jellyfin") {
        return "Jellyfin".to_string();
    }
    
    let is_browser = ["chrome", "msedge", "firefox", "browser", "opera", "brave", "safari"]
        .iter()
        .any(|&b| app_id_lower.contains(b));
        
    if is_browser {
        if title_lower.contains("youtube music") || album_lower.contains("youtube music") || artist_lower.contains("youtube music") {
            return "YouTube Music".to_string();
        } else if title_lower.contains("youtube") || album_lower.contains("youtube") || artist_lower.contains("youtube") {
            return "YouTube".to_string();
        } else if title_lower.contains("netflix") || album_lower.contains("netflix") {
            return "Netflix".to_string();
        } else if title_lower.contains("soundcloud") || album_lower.contains("soundcloud") {
            return "SoundCloud".to_string();
        } else if title_lower.contains("twitch") || album_lower.contains("twitch") {
            return "Twitch".to_string();
        } else if title_lower.contains("jellyfin") || album_lower.contains("jellyfin") {
            return "Jellyfin".to_string();
        }
        
        if app_id_lower.contains("chrome") {
            return "Chrome Browser".to_string();
        } else if app_id_lower.contains("msedge") {
            return "Edge Browser".to_string();
        } else if app_id_lower.contains("firefox") {
            return "Firefox".to_string();
        }
    }
    
    if app_id_lower.contains("backgroundplayback") {
        return "Windows Media".to_string();
    }
    
    let name = app_id.split('\\').last().unwrap_or(app_id).split('/').last().unwrap_or(app_id);
    let mut clean_name = name.to_string();
    if clean_name.to_lowercase().ends_with(".exe") {
        clean_name.truncate(clean_name.len() - 4);
    }
    
    let parts: Vec<&str> = clean_name.split('.').collect();
    if parts.len() > 1 {
        if let Ok(re) = regex::Regex::new(r"^[a-zA-Z0-9]{15,40}$") {
            let filtered: Vec<&str> = parts.into_iter().filter(|&p| !re.is_match(p) && p.to_lowercase() != "exe" && p.to_lowercase() != "app").collect();
            if let Some(&last_part) = filtered.last() {
                clean_name = last_part.to_string();
            }
        }
    }
    
    if let Ok(re_hash) = regex::Regex::new(r"_[a-zA-Z0-9]{8,15}$") {
        clean_name = re_hash.replace(&clean_name, "").to_string();
    }
    if let Ok(re_long) = regex::Regex::new(r"^[a-zA-Z0-9]{15,40}$") {
        clean_name = re_long.replace(&clean_name, "").to_string();
    }
    
    if clean_name.trim().is_empty() {
        return "Windows Media".to_string();
    }
    
    clean_name.trim().to_string()
}

fn parse_lrc(lrc_text: &str) -> Vec<serde_json::Value> {
    let mut lines = Vec::new();
    for line in lrc_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(start_idx) = line.find('[') {
            if let Some(end_idx) = line.find(']') {
                let time_str = &line[start_idx + 1..end_idx];
                let text = line[end_idx + 1..].trim();
                
                let parts: Vec<&str> = time_str.split(':').collect();
                if parts.len() == 2 {
                    if let Ok(minutes) = parts[0].parse::<f64>() {
                        let sec_parts: Vec<&str> = parts[1].split('.').collect();
                        if let Ok(seconds) = sec_parts[0].parse::<f64>() {
                            let ms = if sec_parts.len() == 2 {
                                let ms_str = sec_parts[1];
                                if let Ok(ms_val) = ms_str.parse::<f64>() {
                                    if ms_str.len() == 2 {
                                        ms_val * 10.0
                                    } else {
                                        ms_val
                                    }
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            };
                            let time_sec = minutes * 60.0 + seconds + (ms / 1000.0);
                            lines.push(serde_json::json!({
                                "time": time_sec,
                                "text": text
                            }));
                        }
                    }
                }
            }
        }
    }
    lines.sort_by(|a, b| {
        let a_time = a.get("time").and_then(|t| t.as_f64()).unwrap_or(0.0);
        let b_time = b.get("time").and_then(|t| t.as_f64()).unwrap_or(0.0);
        a_time.partial_cmp(&b_time).unwrap_or(std::cmp::Ordering::Equal)
    });
    lines
}

async fn fetch_lrclib_lyrics(artist: &str, title: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .user_agent("OBS-Now-Playing-Overlay/1.0")
        .build()
        .ok()?;
        
    let c_artist = clean_metadata_for_search(artist);
    let c_title = clean_metadata_for_search(title);
    
    // 1. Try exact get
    let url = "https://lrclib.net/api/get";
    if let Ok(resp) = client.get(url)
        .query(&[("artist_name", &c_artist), ("track_name", &c_title)])
        .send()
        .await 
    {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(synced) = json.get("syncedLyrics").and_then(|s| s.as_str()) {
                    if !synced.is_empty() {
                        return Some(synced.to_string());
                    }
                }
                if let Some(plain) = json.get("plainLyrics").and_then(|p| p.as_str()) {
                    if !plain.is_empty() {
                        return Some(plain.to_string());
                    }
                }
            }
        }
    }
    
    // 2. Try search list
    let search_url = "https://lrclib.net/api/search";
    if let Ok(resp) = client.get(search_url)
        .query(&[("q", &format!("{} {}", c_artist, c_title))])
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(results) = resp.json::<Vec<serde_json::Value>>().await {
                for item in results {
                    if let Some(synced) = item.get("syncedLyrics").and_then(|s| s.as_str()) {
                        if !synced.is_empty() {
                            return Some(synced.to_string());
                        }
                    }
                    if let Some(plain) = item.get("plainLyrics").and_then(|p| p.as_str()) {
                        if !plain.is_empty() {
                            return Some(plain.to_string());
                        }
                    }
                }
            }
        }
    }
    
    None
}

async fn fetch_genius_lyrics(artist: &str, title: &str, duration: f64) -> Option<Vec<serde_json::Value>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .ok()?;
    let query = format!("{} {}", artist, title);
    let search_url = "https://genius.com/api/search/multi";
    
    let resp = client.get(search_url).query(&[("q", &query)]).send().await.ok()?;
    let json = resp.json::<serde_json::Value>().await.ok()?;
    let sections = json.get("response")?.get("sections")?.as_array()?;
    
    let mut song_url = None;
    for section in sections {
        if let Some(hits) = section.get("hits").and_then(|h| h.as_array()) {
            for hit in hits {
                if hit.get("type").and_then(|t| t.as_str()) == Some("song") {
                    if let Some(url) = hit.get("result").and_then(|r| r.get("url")).and_then(|u| u.as_str()) {
                        song_url = Some(url.to_string());
                        break;
                    }
                }
            }
        }
        if song_url.is_some() { break; }
    }
    
    let url = song_url?;
    let page_resp = client.get(&url).send().await.ok()?;
    let html = page_resp.text().await.ok()?;
    
    let re_container = regex::Regex::new(r#"(?s)<div[^>]*data-lyrics-container="true"[^>]*>(.*?)</div>"#).ok()?;
    let mut raw_text = String::new();
    for cap in re_container.captures_iter(&html) {
        raw_text.push_str(&cap[1]);
        raw_text.push('\n');
    }
    
    if raw_text.is_empty() {
        return None;
    }
    
    let re_tags = regex::Regex::new(r"<[^>]*>").ok()?;
    let mut cleaned_text = re_tags.replace_all(&raw_text, "\n").to_string();
    cleaned_text = cleaned_text.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#x27;", "'");
    
    let mut lines = Vec::new();
    for line in cleaned_text.lines() {
        let line_str = line.trim();
        if !line_str.is_empty() && !(line_str.starts_with('[') && line_str.ends_with(']')) {
            lines.push(line_str.to_string());
        }
    }
    
    if lines.is_empty() {
        return None;
    }
    
    let dur = if duration > 20.0 { duration } else { 180.0 };
    let interval = (dur - 10.0) / lines.len() as f64;
    let mut parsed = Vec::new();
    for (i, line) in lines.into_iter().enumerate() {
        let t = i as f64 * interval;
        parsed.push(serde_json::json!({
            "time": (t * 100.0).round() / 100.0,
            "text": line
        }));
    }
    
    Some(parsed)
}

async fn fetch_release_year(artist: &str, title: &str) -> String {
    let client = reqwest::Client::new();
    let query = format!("{} {}", clean_metadata_for_search(artist), clean_metadata_for_search(title));
    if let Ok(resp) = client.get("https://itunes.apple.com/search")
        .query(&[("term", query.as_str()), ("media", "music"), ("limit", "1")])
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(results) = json.get("results").and_then(|r| r.as_array()) {
                    if !results.is_empty() {
                        if let Some(release_date) = results[0].get("releaseDate").and_then(|d| d.as_str()) {
                            if release_date.len() >= 4 {
                                return release_date[..4].to_string();
                            }
                        }
                    }
                }
            }
        }
    }
    "".to_string()
}

fn fetch_thumbnail_sync(props: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties) -> Option<String> {
    use windows::Storage::Streams::{Buffer, DataReader, InputStreamOptions};
    let thumb_ref = props.Thumbnail().ok()?;
    let op = thumb_ref.OpenReadAsync().ok()?;
    let stream = op.get().ok()?;
    let size = stream.Size().unwrap_or(0) as u32;
    if size == 0 {
        return None;
    }
    let buffer = Buffer::Create(size).ok()?;
    let read_op = stream.ReadAsync(&buffer, size, InputStreamOptions::None).ok()?;
    read_op.get().ok()?;
    let reader = DataReader::FromBuffer(&buffer).ok()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    use base64::prelude::*;
    Some(BASE64_STANDARD.encode(&bytes))
}

async fn fetch_windows_media_state() -> Option<TrackState> {
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;
    
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?.await.ok()?;
    let session = manager.GetCurrentSession().ok()?;
    
    let props = session.TryGetMediaPropertiesAsync().ok()?.await.ok()?;
    let title = props.Title().unwrap_or_default().to_string();
    let artist = props.Artist().unwrap_or_default().to_string();
    let album = props.AlbumTitle().unwrap_or_default().to_string();
    
    let pb_info = session.GetPlaybackInfo().ok()?;
    let status_val = pb_info.PlaybackStatus().ok()?.0;
    
    let playback_status = match status_val {
        4 => "playing".to_string(),
        5 => "paused".to_string(),
        _ => "stopped".to_string(),
    };
    
    let tl_props = session.GetTimelineProperties().ok();
    let position = if let Some(ref tl) = tl_props {
        if let Ok(pos) = tl.Position() {
            pos.Duration as f64 / 10_000_000.0
        } else {
            0.0
        }
    } else {
        0.0
    };
    let duration = if let Some(ref tl) = tl_props {
        if let Ok(end) = tl.EndTime() {
            end.Duration as f64 / 10_000_000.0
        } else {
            0.0
        }
    } else {
        0.0
    };
    
    let app_id = session.SourceAppUserModelId().unwrap_or_default().to_string();
    let source = get_readable_source(&app_id, &title, &artist, &album);
    
    let thumbnail = fetch_thumbnail_sync(&props).unwrap_or_default();
    
    Some(TrackState {
        title,
        artist,
        album,
        playback_status,
        position,
        duration,
        source,
        thumbnail,
        source_app_id: app_id,
        lyrics: Vec::new(),
        year: String::new(),
    })
}

fn broadcast_now_playing_state(state: &Arc<AppState>) {
    let current_state = state.nowplaying_state.lock().unwrap().clone();
    let cfg = state.config.lock().unwrap().clone();
    let payload = serde_json::json!({
        "state": current_state,
        "config": {
            "features": {
                "vinyl_enabled": cfg.tools.nowplaying.settings.vinyl_enabled,
                "lyrics_enabled": cfg.tools.nowplaying.settings.lyrics_enabled,
                "visualizer_enabled": cfg.tools.nowplaying.settings.visualizer_enabled,
                "hide_on_alert": cfg.tools.nowplaying.settings.hide_on_alert,
                "detect_spotify": cfg.tools.nowplaying.settings.detect_spotify,
                "detect_chrome": cfg.tools.nowplaying.settings.detect_chrome,
                "detect_firefox": cfg.tools.nowplaying.settings.detect_firefox,
                "detect_system": cfg.tools.nowplaying.settings.detect_system,
            },
            "styling": {
                "font_family": cfg.tools.nowplaying.settings.font_family,
                "font_size": cfg.tools.nowplaying.settings.font_size,
                "color_title": cfg.tools.nowplaying.settings.color_title,
                "color_artist": cfg.tools.nowplaying.settings.color_artist,
                "color_bg": cfg.tools.nowplaying.settings.color_bg,
                "bg_opacity": cfg.tools.nowplaying.settings.bg_opacity,
                "border_color": cfg.tools.nowplaying.settings.border_color,
                "border_width": cfg.tools.nowplaying.settings.border_width,
                "border_radius": cfg.tools.nowplaying.settings.border_radius,
                "scroll_text": cfg.tools.nowplaying.settings.scroll_text,
                "padding_px": cfg.tools.nowplaying.settings.padding_px,
                "theme": cfg.tools.nowplaying.settings.theme,
                "accent_color": cfg.tools.nowplaying.settings.accent_color,
            }
        }
    });
    let _ = state.ws_tx.send(payload.to_string());
}

fn start_now_playing_listener(state: Arc<AppState>) {
    let sse_tx = state.sse_tx.clone();
    let state_clone = state.clone();
    
    tokio::spawn(async move {
        loop {
            let config_locked = state_clone.config.lock().unwrap().clone();
            
            if config_locked.tools.nowplaying.enabled {
                if let Some(fetched) = fetch_windows_media_state().await {
                    let settings = &config_locked.tools.nowplaying.settings;
                    let app_id = fetched.source_app_id.to_lowercase();
                    
                    let passes_filter = (settings.detect_spotify && app_id.contains("spotify")) ||
                                        (settings.detect_chrome && app_id.contains("chrome")) ||
                                        (settings.detect_firefox && app_id.contains("firefox")) ||
                                        (settings.detect_system && !app_id.contains("spotify") && !app_id.contains("chrome") && !app_id.contains("firefox"));
                                        
                    if passes_filter {
                        let mut current_state = state_clone.nowplaying_state.lock().unwrap();
                        
                        let track_changed = fetched.title != current_state.title || fetched.artist != current_state.artist;
                        let status_changed = fetched.playback_status != current_state.playback_status;
                        let pos_jumped = (fetched.position - current_state.position).abs() > 1.5;
                        let thumb_changed = fetched.thumbnail != current_state.thumbnail;
                        
                        if track_changed {
                            current_state.title = fetched.title.clone();
                            current_state.artist = fetched.artist.clone();
                            current_state.album = fetched.album.clone();
                            current_state.playback_status = fetched.playback_status.clone();
                            current_state.position = fetched.position;
                            current_state.duration = fetched.duration;
                            current_state.source = fetched.source.clone();
                            current_state.thumbnail = fetched.thumbnail.clone();
                            current_state.source_app_id = fetched.source_app_id.clone();
                            current_state.year = String::new();
                            current_state.lyrics = Vec::new();
                            
                            drop(current_state);
                            broadcast_now_playing_state(&state_clone);
                            
                            let _ = sse_tx.send(serde_json::json!({
                                "event": "now_playing",
                                "data": {
                                    "title": fetched.title.clone(),
                                    "artist": fetched.artist.clone(),
                                    "app": fetched.source_app_id.clone()
                                }
                            }));
                            
                            let state_bg = state_clone.clone();
                            let title_bg = fetched.title.clone();
                            let artist_bg = fetched.artist.clone();
                            let duration_bg = fetched.duration;
                            
                            tokio::spawn(async move {
                                let lyrics_enabled = state_bg.config.lock().unwrap().tools.nowplaying.settings.lyrics_enabled;
                                 let mut parsed_lyrics = Vec::new();
                                
                                let year_fut = fetch_release_year(&artist_bg, &title_bg);
                                if lyrics_enabled {
                                    if let Some(lrc_text) = fetch_lrclib_lyrics(&artist_bg, &title_bg).await {
                                        parsed_lyrics = parse_lrc(&lrc_text);
                                    } else if let Some(gen_lyrics) = fetch_genius_lyrics(&artist_bg, &title_bg, duration_bg).await {
                                        parsed_lyrics = gen_lyrics;
                                    }
                                }
                                let year = year_fut.await;
                                
                                {
                                    let mut current = state_bg.nowplaying_state.lock().unwrap();
                                    if current.title == title_bg && current.artist == artist_bg {
                                        current.year = year;
                                        current.lyrics = parsed_lyrics;
                                    }
                                }
                                broadcast_now_playing_state(&state_bg);
                            });
                        } else if status_changed || pos_jumped || thumb_changed {
                            current_state.playback_status = fetched.playback_status;
                            current_state.position = fetched.position;
                            current_state.duration = fetched.duration;
                            current_state.thumbnail = fetched.thumbnail;
                            
                            drop(current_state);
                            broadcast_now_playing_state(&state_clone);
                        } else {
                            current_state.position = fetched.position;
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}

// Twitch EventSub WebSocket Connection
fn map_eventsub_notification_to_alert(event_type: &str, event_data: &serde_json::Value) -> Option<serde_json::Value> {
    let name = event_data.get("user_name").and_then(|n| n.as_str()).unwrap_or("Someone").to_string();
    
    if event_type == "channel.follow" {
        Some(serde_json::json!({
            "type": "follow",
            "name": name,
            "value": ""
        }))
    } else if event_type == "channel.subscribe" {
        let tier_raw = event_data.get("tier").and_then(|t| t.as_str()).unwrap_or("1000");
        let tier = match tier_raw {
            "2000" => "Tier 2",
            "3000" => "Tier 3",
            _ => "Tier 1",
        };
        Some(serde_json::json!({
            "type": "sub",
            "name": name,
            "value": tier
        }))
    } else if event_type == "channel.subscription.gift" {
        let total = event_data.get("total").and_then(|t| t.as_u64()).unwrap_or(1);
        Some(serde_json::json!({
            "type": "sub",
            "name": name,
            "value": format!("Gifted {} subs!", total)
        }))
    } else if event_type == "channel.cheer" {
        let bits = event_data.get("bits").and_then(|b| b.as_u64()).unwrap_or(0);
        Some(serde_json::json!({
            "type": "donation",
            "name": name,
            "value": format!("{} Bits", bits)
        }))
    } else if event_type == "channel.raid" {
        let name_from = event_data.get("from_broadcaster_user_name").and_then(|n| n.as_str()).unwrap_or("Someone").to_string();
        let viewers = event_data.get("viewers").and_then(|v| v.as_u64()).unwrap_or(0);
        Some(serde_json::json!({
            "type": "donation",
            "name": name_from,
            "value": format!("Raid with {} viewers!", viewers)
        }))
    } else {
        None
    }
}

async fn subscribe_eventsub_topics(tokens: &TwitchTokens, session_id: &str) -> bool {
    let client = reqwest::Client::new();
    let headers = {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert("Authorization", format!("Bearer {}", tokens.access_token).parse().unwrap());
        h.insert("Client-Id", TWITCH_CLIENT_ID.parse().unwrap());
        h.insert("Content-Type", "application/json".parse().unwrap());
        h
    };
    
    let user_id = &tokens.user_id;
    let subscriptions = vec![
        serde_json::json!({
            "type": "channel.follow",
            "version": "2",
            "condition": {"broadcaster_user_id": user_id, "moderator_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        }),
        serde_json::json!({
            "type": "channel.subscribe",
            "version": "1",
            "condition": {"broadcaster_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        }),
        serde_json::json!({
            "type": "channel.subscription.gift",
            "version": "1",
            "condition": {"broadcaster_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        }),
        serde_json::json!({
            "type": "channel.cheer",
            "version": "1",
            "condition": {"broadcaster_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        }),
        serde_json::json!({
            "type": "channel.raid",
            "version": "1",
            "condition": {"to_broadcaster_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        }),
        serde_json::json!({
            "type": "channel.poll.begin",
            "version": "1",
            "condition": {"broadcaster_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        }),
        serde_json::json!({
            "type": "channel.poll.progress",
            "version": "1",
            "condition": {"broadcaster_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        }),
        serde_json::json!({
            "type": "channel.poll.end",
            "version": "1",
            "condition": {"broadcaster_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        }),
        serde_json::json!({
            "type": "channel.prediction.begin",
            "version": "1",
            "condition": {"broadcaster_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        }),
        serde_json::json!({
            "type": "channel.prediction.progress",
            "version": "1",
            "condition": {"broadcaster_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        }),
        serde_json::json!({
            "type": "channel.prediction.lock",
            "version": "1",
            "condition": {"broadcaster_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        }),
        serde_json::json!({
            "type": "channel.prediction.end",
            "version": "1",
            "condition": {"broadcaster_user_id": user_id},
            "transport": {"method": "websocket", "session_id": session_id}
        })
    ];
    
    let mut any_unauthorized = false;
    for sub in subscriptions {
        let resp = client.post("https://api.twitch.tv/helix/eventsub/subscriptions")
            .headers(headers.clone())
            .json(&sub)
            .send()
            .await;
            
        match resp {
            Ok(r) => {
                let status = r.status();
                println!("[TWITCH] Subscribed to {} -> status: {}", sub["type"], status);
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    any_unauthorized = true;
                }
            }
            Err(e) => {
                println!("[TWITCH] Subscription request failed for {}: {}", sub["type"], e);
            }
        }
    }
    any_unauthorized
}

async fn run_twitch_eventsub_loop(state: Arc<AppState>) {
    loop {
        let tokens = load_twitch_tokens();
        if tokens.access_token.is_empty() {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }
        
        println!("[TWITCH] Connecting to Twitch EventSub WebSocket...");
        let url = "wss://eventsub.wss.twitch.tv/ws";
        
        match tokio_tungstenite::connect_async(url).await {
            Ok((mut ws_stream, _)) => {
                println!("[TWITCH] EventSub WebSocket connected!");
                
                while let Some(Ok(msg)) = ws_stream.next().await {
                    if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                            let msg_type = val.get("metadata").and_then(|m| m.get("message_type")).and_then(|t| t.as_str()).unwrap_or("");
                            
                            if msg_type == "session_welcome" {
                                if let Some(session_id) = val.get("payload").and_then(|p| p.get("session")).and_then(|s| s.get("id")).and_then(|i| i.as_str()) {
                                    println!("[TWITCH] Session welcome received. ID: {}", session_id);
                                    let unauthorized = subscribe_eventsub_topics(&tokens, session_id).await;
                                    if unauthorized {
                                        println!("[TWITCH] Session credentials unauthorized. Expiring local session...");
                                        {
                                            let mut cfg = state.config.lock().unwrap();
                                            cfg.twitch_connected = false;
                                            save_config(&cfg);
                                        }
                                        save_twitch_tokens(&TwitchTokens::default());
                                        
                                        // Broadcast config update to frontend clients
                                        let updated_cfg = state.config.lock().unwrap().clone();
                                        let _ = state.sse_tx.send(serde_json::json!({
                                            "event": "config_update",
                                            "data": updated_cfg
                                        }));
                                    }
                                }
                            } else if msg_type == "notification" {
                                let event_type = val.get("payload").and_then(|p| p.get("subscription")).and_then(|s| s.get("type")).and_then(|t| t.as_str()).unwrap_or("");
                                if let Some(event_data) = val.get("payload").and_then(|p| p.get("event")) {
                                    // Comprobar si el evento es de encuestas o predicciones reales de Twitch
                                    if event_type.starts_with("channel.poll.") || event_type.starts_with("channel.prediction.") {
                                        let ws_event = match event_type {
                                            "channel.poll.begin" => "poll_begin",
                                            "channel.poll.progress" => "poll_progress",
                                            "channel.poll.end" => "poll_end",
                                            "channel.prediction.begin" => "prediction_begin",
                                            "channel.prediction.progress" => "prediction_progress",
                                            "channel.prediction.lock" => "prediction_lock",
                                            "channel.prediction.end" => "prediction_end",
                                            _ => "",
                                        };
                                        
                                        if !ws_event.is_empty() {
                                            let ws_payload = serde_json::json!({
                                                "event": ws_event,
                                                "data": event_data
                                            });
                                            let _ = state.ws_tx.send(ws_payload.to_string());
                                        }
                                    } else if let Some(alert) = map_eventsub_notification_to_alert(event_type, event_data) {
                                        // Broadcast the alert to ws_tx
                                        let ws_payload = serde_json::json!({
                                            "event": "alert",
                                            "data": alert
                                        });
                                        let _ = state.ws_tx.send(ws_payload.to_string());
                                        
                                        // Also broadcast to sse_tx for standard alerts
                                        let alert_type_str = alert["type"].as_str().unwrap_or("follow");
                                        let duration_ms = {
                                            let cfg = state.config.lock().unwrap();
                                            match alert_type_str {
                                                "sub"  => cfg.tools.alerts.settings.sub_duration_ms,
                                                "raid" => cfg.tools.alerts.settings.raid_duration_ms,
                                                _      => cfg.tools.alerts.settings.follow_duration_ms,
                                            }
                                        };
                                        let _ = state.sse_tx.send(serde_json::json!({
                                            "event": "twitch_alert",
                                            "data": {
                                                "type": alert["type"],
                                                "user": alert["name"],
                                                "count": 0,
                                                "duration_ms": duration_ms
                                            }
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("[TWITCH] EventSub connection error: {}. Retrying in 5s...", e);
            }
        }
        
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

// Axum Web Routing
async fn get_dashboard(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let connected = {
        let cfg = state.config.lock().unwrap();
        cfg.twitch_connected
    };
    
    if !connected {
        return axum::response::Redirect::temporary("/twitch/login?expired=true").into_response();
    }
    
    axum::response::Html(include_str!("../templates/index.html")).into_response()
}

async fn get_twitch_login_page() -> impl IntoResponse {
    axum::response::Html(include_str!("../templates/twitch.html"))
}

async fn get_chat_overlay() -> impl IntoResponse {
    axum::response::Html(include_str!("../overlays/chat.html"))
}

async fn get_carousel_overlay() -> impl IntoResponse {
    axum::response::Html(include_str!("../overlays/carousel.html"))
}

async fn get_nowplaying_overlay() -> impl IntoResponse {
    axum::response::Html(include_str!("../overlays/nowplaying.html"))
}

async fn get_nowplaying_css() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        include_str!("../overlays/nowplaying.css"),
    )
        .into_response()
}

async fn get_nowplaying_js() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        include_str!("../overlays/nowplaying.js"),
    )
        .into_response()
}

async fn get_alerts_overlay() -> impl IntoResponse {
    axum::response::Html(include_str!("../overlays/alerts.html"))
}

async fn get_alerts_css() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        include_str!("../overlays/alerts.css"),
    )
        .into_response()
}

async fn get_alerts_js() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        include_str!("../overlays/alerts.js"),
    )
        .into_response()
}

async fn get_polls_overlay() -> impl IntoResponse {
    axum::response::Html(include_str!("../overlays/polls.html"))
}

async fn get_polls_css() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        include_str!("../overlays/polls.css"),
    )
        .into_response()
}

async fn get_polls_js() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        include_str!("../overlays/polls.js"),
    )
        .into_response()
}

async fn get_worldcup_overlay() -> impl IntoResponse {
    axum::response::Html(include_str!("../overlays/worldcup.html"))
}

async fn get_worldcup_css() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        include_str!("../overlays/worldcup.css"),
    )
        .into_response()
}

async fn get_worldcup_js() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        include_str!("../overlays/worldcup.js"),
    )
        .into_response()
}



#[derive(Deserialize)]
struct UploadPayload {
    filename: String,
    content: String,
}

async fn post_upload(Json(payload): Json<UploadPayload>) -> impl IntoResponse {
    use base64::prelude::*;
    let bytes = match BASE64_STANDARD.decode(&payload.content) {
        Ok(b) => b,
        Err(_) => return (axum::http::StatusCode::BAD_REQUEST, "Invalid base64").into_response(),
    };
    
    let uploads_dir = resolve_path("uploads");
    if !uploads_dir.exists() {
        let _ = fs::create_dir_all(&uploads_dir);
    }
    
    let cleaned_filename = PathBuf::from(&payload.filename)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
        
    if cleaned_filename.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Invalid filename").into_response();
    }
    
    let unique_filename = format!("{}_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), cleaned_filename);
    let file_path = uploads_dir.join(&unique_filename);
    
    if fs::write(&file_path, bytes).is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to write file").into_response();
    }
    
    let url = format!("/uploads/{}", unique_filename);
    Json(serde_json::json!({ "url": url })).into_response()
}

async fn get_upload_file(axum::extract::Path(filename): axum::extract::Path<String>) -> impl IntoResponse {
    let uploads_dir = resolve_path("uploads");
    let path = uploads_dir.join(&filename);
    
    match fs::read(path) {
        Ok(bytes) => {
            let mime = if filename.ends_with(".gif") {
                "image/gif"
            } else if filename.ends_with(".png") {
                "image/png"
            } else if filename.ends_with(".mp4") {
                "video/mp4"
            } else {
                "image/jpeg"
            };
            ([(axum::http::header::CONTENT_TYPE, mime)], bytes).into_response()
        }
        Err(_) => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}

async fn get_api_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cfg = state.config.lock().unwrap().clone();
    Json(cfg)
}

async fn post_api_config(
    State(state): State<Arc<AppState>>,
    Json(new_cfg): Json<Config>,
) -> impl IntoResponse {
    {
        let mut cfg = state.config.lock().unwrap();
        *cfg = new_cfg.clone();
        save_config(&cfg);
    }
    
    let _ = state.sse_tx.send(serde_json::json!({
        "event": "config_update",
        "data": new_cfg
    }));
    
    // Broadcast websocket config_update to trigger reload on WS-based overlays
    let ws_msg = serde_json::json!({
        "event": "config_update"
    });
    let _ = state.ws_tx.send(ws_msg.to_string());
    
    broadcast_now_playing_state(&state);
    start_twitch_chat(state.clone());
    Json(serde_json::json!({"status": "success"}))
}

async fn post_twitch_login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let channel = body.get("channel").and_then(|c| c.as_str()).unwrap_or("").to_string();
    if channel.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"status": "error", "message": "Channel required"})));
    }
    
    let updated_cfg = {
        let mut cfg = state.config.lock().unwrap();
        cfg.twitch_channel = channel.clone();
        cfg.twitch_username = channel;
        cfg.twitch_connected = true;
        save_config(&cfg);
        cfg.clone()
    };
    
    let _ = state.sse_tx.send(serde_json::json!({
        "event": "config_update",
        "data": updated_cfg
    }));
    
    start_twitch_chat(state.clone());
    (axum::http::StatusCode::OK, Json(serde_json::json!({"status": "success"})))
}

async fn post_twitch_logout(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let updated_cfg = {
        let mut cfg = state.config.lock().unwrap();
        cfg.twitch_channel = "".to_string();
        cfg.twitch_username = "".to_string();
        cfg.twitch_connected = false;
        save_config(&cfg);
        cfg.clone()
    };
    
    let _ = state.sse_tx.send(serde_json::json!({
        "event": "config_update",
        "data": updated_cfg
    }));
    
    if let Some(h) = state.twitch_handle.lock().unwrap().take() {
        h.abort();
    }
    
    Json(serde_json::json!({"status": "success"}))
}

async fn get_twitch_auth() -> impl IntoResponse {
    let auth_url = format!(
        "https://id.twitch.tv/oauth2/authorize?client_id={}&redirect_uri=http%3A%2F%2Flocalhost%3A777%2Ftwitch%2Fcallback&response_type=code&scope=moderator:read:followers+channel:read:subscriptions+bits:read+channel:read:polls+channel:read:predictions",
        TWITCH_CLIENT_ID
    );
    axum::response::Redirect::temporary(&auth_url)
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
}

async fn get_twitch_callback(
    axum::extract::Query(query): axum::extract::Query<CallbackQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let code = match query.code {
        Some(c) => c,
        None => return axum::response::Html("<h1>Error: Code missing</h1>".to_string()).into_response(),
    };
    
    let client = reqwest::Client::new();
    let token_resp = client.post("https://id.twitch.tv/oauth2/token")
        .form(&[
            ("client_id", TWITCH_CLIENT_ID),
            ("client_secret", TWITCH_CLIENT_SECRET),
            ("code", &code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", TWITCH_REDIRECT_URI),
        ])
        .send()
        .await;
        
    let token_json = match token_resp {
        Ok(resp) => {
            if resp.status().is_success() {
                resp.json::<serde_json::Value>().await.unwrap_or(serde_json::Value::Null)
            } else {
                return axum::response::Html(format!("<h1>Token exchange failed</h1><p>{}</p>", resp.text().await.unwrap_or_default())).into_response();
            }
        }
        Err(e) => return axum::response::Html(format!("<h1>Token request failed</h1><p>{}</p>", e)).into_response(),
    };
    
    let access_token = token_json.get("access_token").and_then(|a| a.as_str()).unwrap_or("").to_string();
    let refresh_token = token_json.get("refresh_token").and_then(|r| r.as_str()).unwrap_or("").to_string();
    
    if access_token.is_empty() {
        return axum::response::Html("<h1>Error: Access token empty</h1>".to_string()).into_response();
    }
    
    let user_resp = client.get("https://api.twitch.tv/helix/users")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Client-Id", TWITCH_CLIENT_ID)
        .send()
        .await;
        
    let mut user_id = String::new();
    let mut user_login = String::new();
    if let Ok(resp) = user_resp {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                if !data.is_empty() {
                    user_id = data[0].get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
                    user_login = data[0].get("login").and_then(|l| l.as_str()).unwrap_or("").to_string();
                }
            }
        }
    }
    
    let tokens = TwitchTokens {
        access_token,
        refresh_token,
        user_id,
        user_login: user_login.clone(),
    };
    save_twitch_tokens(&tokens);
    
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.twitch_channel = user_login.clone();
        cfg.twitch_username = user_login.clone();
        cfg.twitch_connected = true;
        save_config(&cfg);
    }
    
    let state_clone = state.clone();
    tokio::spawn(async move {
        run_twitch_eventsub_loop(state_clone).await;
    });
    
    axum::response::Html(format!(
        r#"
        <html>
        <head>
            <title>Twitch EventSub Connected</title>
            <script>
                setTimeout(function() {{
                    window.location.href = "/";
                }}, 3000);
            </script>
        </head>
        <body style="background:#0f0f14;color:#fff;font-family:sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;margin:0">
        <div style="text-align:center">
            <h1 style="color:#10b981">✅ Twitch EventSub Connected!</h1>
            <p style="color:rgba(255,255,255,0.7)">Logged in as <strong style="color:#a855f7">{}</strong></p>
            <p style="color:rgba(255,255,255,0.5)">Redirigiéndote al panel de control en unos momentos...</p>
        </div>
        </body>
        </html>
        "#,
        user_login
    )).into_response()
}

async fn post_trigger_alert(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let alert_type = body.get("alert_type").and_then(|a| a.as_str()).unwrap_or("follow");
    let user = body.get("user").and_then(|u| u.as_str()).unwrap_or("NovaViewer");
    let count = body.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
    
    let duration_ms = {
        let cfg = state.config.lock().unwrap();
        match alert_type {
            "sub"  => cfg.tools.alerts.settings.sub_duration_ms,
            "raid" => cfg.tools.alerts.settings.raid_duration_ms,
            _      => cfg.tools.alerts.settings.follow_duration_ms,
        }
    };

    let _ = state.sse_tx.send(serde_json::json!({
        "event": "twitch_alert",
        "data": {
            "type": alert_type,
            "user": user,
            "count": count,
            "duration_ms": duration_ms
        }
    }));
    
    let ws_val = if alert_type == "sub" { format!("Tier {}", count) } else { format!("{}", count) };
    let ws_payload = serde_json::json!({
        "event": "alert",
        "data": {
            "type": alert_type,
            "name": user,
            "value": ws_val
        }
    });
    let _ = state.ws_tx.send(ws_payload.to_string());
    
    Json(serde_json::json!({"status": "success"}))
}

async fn post_trigger_chat_message(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let user = body.get("user").and_then(|u| u.as_str()).unwrap_or("TestUser");
    let message = body.get("message").and_then(|m| m.as_str()).unwrap_or("");
    let badges = body.get("badges").and_then(|b| b.as_array()).cloned().unwrap_or_default();
    let color = body.get("color").and_then(|c| c.as_str()).unwrap_or("#00f0ff");
    let id = format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    
    let msg_data = serde_json::json!({
        "user": user,
        "message": message,
        "badges": badges,
        "color": color,
        "id": id
    });
    
    let _ = state.sse_tx.send(serde_json::json!({
        "event": "chat_message",
        "data": msg_data
    }));
    
    Json(serde_json::json!({"status": "success"}))
}

async fn handle_sse(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.sse_tx.subscribe();
    
    let current_cfg = state.config.lock().unwrap().clone();
    let initial_event = Event::default()
        .event("initial_config")
        .data(serde_json::to_string(&current_cfg).unwrap());
        
    let initial_stream = stream::once(async move { Ok(initial_event) });
    
    let event_stream = stream::unfold(rx, |mut rx| async move {
        loop {
            if let Ok(msg) = rx.recv().await {
                let event_type = msg.get("event").and_then(|e| e.as_str()).unwrap_or("message");
                let data = msg.get("data").unwrap().to_string();
                let event = Event::default().event(event_type).data(data);
                return Some((Ok(event), rx));
            }
        }
    });
    
    let combined = stream::select(initial_stream, event_stream);
    Sse::new(combined).keep_alive(KeepAlive::default())
}

async fn get_obs_nowplaying_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let initial_payload = {
        let current_state = state.nowplaying_state.lock().unwrap().clone();
        let cfg = state.config.lock().unwrap().clone();
        serde_json::json!({
            "state": current_state,
            "config": {
                "features": {
                    "vinyl_enabled": cfg.tools.nowplaying.settings.vinyl_enabled,
                    "lyrics_enabled": cfg.tools.nowplaying.settings.lyrics_enabled,
                    "visualizer_enabled": cfg.tools.nowplaying.settings.visualizer_enabled,
                    "hide_on_alert": cfg.tools.nowplaying.settings.hide_on_alert,
                    "detect_spotify": cfg.tools.nowplaying.settings.detect_spotify,
                    "detect_chrome": cfg.tools.nowplaying.settings.detect_chrome,
                    "detect_firefox": cfg.tools.nowplaying.settings.detect_firefox,
                    "detect_system": cfg.tools.nowplaying.settings.detect_system,
                },
                "styling": {
                    "font_family": cfg.tools.nowplaying.settings.font_family,
                    "font_size": cfg.tools.nowplaying.settings.font_size,
                    "color_title": cfg.tools.nowplaying.settings.color_title,
                    "color_artist": cfg.tools.nowplaying.settings.color_artist,
                    "color_bg": cfg.tools.nowplaying.settings.color_bg,
                    "bg_opacity": cfg.tools.nowplaying.settings.bg_opacity,
                    "border_color": cfg.tools.nowplaying.settings.border_color,
                    "border_width": cfg.tools.nowplaying.settings.border_width,
                    "border_radius": cfg.tools.nowplaying.settings.border_radius,
                    "scroll_text": cfg.tools.nowplaying.settings.scroll_text,
                    "padding_px": cfg.tools.nowplaying.settings.padding_px,
                    "theme": cfg.tools.nowplaying.settings.theme,
                    "accent_color": cfg.tools.nowplaying.settings.accent_color,
                }
            }
        })
    };
    
    let (mut sender, receiver) = socket.split();
    if sender.send(Message::Text(initial_payload.to_string())).await.is_err() {
        return;
    }
    
    let mut rx = state.ws_tx.subscribe();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });
    
    let mut receiver_clone = receiver;
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(_)) = receiver_clone.next().await {}
    });
    
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

async fn get_obs_polls_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_polls_socket(socket, state))
}

async fn handle_polls_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, receiver) = socket.split();
    let mut rx = state.ws_tx.subscribe();
    
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });
    
    let mut receiver_clone = receiver;
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(_)) = receiver_clone.next().await {}
    });
    
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

async fn post_trigger_poll(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    if body.get("event").is_some() {
        let _ = state.ws_tx.send(body.to_string());
    }
    Json(serde_json::json!({"status": "success"}))
}


#[tokio::main]
async fn main() {
    let config = load_config();
    let (sse_tx, _) = broadcast::channel(100);
    let (ws_tx, _) = broadcast::channel(100);
    
    let state = Arc::new(AppState {
        config: Mutex::new(config),
        sse_tx,
        ws_tx,
        twitch_handle: Mutex::new(None),
        nowplaying_state: Mutex::new(TrackState::default()),
    });
    
    // Start listeners
    start_twitch_chat(state.clone());
    start_now_playing_listener(state.clone());
    
    // Start Twitch EventSub listener if tokens exist
    let tokens = load_twitch_tokens();
    if !tokens.access_token.is_empty() {
        let state_clone = state.clone();
        tokio::spawn(async move {
            run_twitch_eventsub_loop(state_clone).await;
        });
    }
    
    // Setup Axum Router
    let app = Router::new()
        .route("/", get(get_dashboard))
        .route("/twitch/login", get(get_twitch_login_page))
        .route("/overlays/chat", get(get_chat_overlay))
        .route("/overlays/carousel", get(get_carousel_overlay))
        .route("/overlays/nowplaying", get(get_nowplaying_overlay))
        .route("/overlays/nowplaying.css", get(get_nowplaying_css))
        .route("/overlays/nowplaying.js", get(get_nowplaying_js))
        .route("/overlays/alerts", get(get_alerts_overlay))
        .route("/overlays/alerts.css", get(get_alerts_css))
        .route("/overlays/alerts.js", get(get_alerts_js))
        .route("/overlays/polls", get(get_polls_overlay))
        .route("/overlays/polls.css", get(get_polls_css))
        .route("/overlays/polls.js", get(get_polls_js))
        .route("/overlays/worldcup", get(get_worldcup_overlay))
        .route("/overlays/worldcup.css", get(get_worldcup_css))
        .route("/overlays/worldcup.js", get(get_worldcup_js))
        .route("/obs/nowplaying/ws", get(get_obs_nowplaying_ws))
        .route("/obs/polls/ws", get(get_obs_polls_ws))
        .route("/api/config", get(get_api_config).post(post_api_config))
        .route("/api/twitch/login", post(post_twitch_login))
        .route("/api/twitch/logout", post(post_twitch_logout))
        .route("/api/trigger-poll", post(post_trigger_poll))
        .route("/twitch/auth", get(get_twitch_auth))
        .route("/twitch/callback", get(get_twitch_callback))
        .route("/api/upload", post(post_upload))
        .route("/uploads/:filename", get(get_upload_file))
        .route("/api/trigger-alert", post(post_trigger_alert))
        .route("/api/trigger-chat-message", post(post_trigger_chat_message))
        .route("/api/stream", get(handle_sse))
        .layer(CorsLayer::permissive())
        .with_state(state.clone());
        
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], PORT));
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        println!("Server running on http://127.0.0.1:{}", PORT);
        axum::serve(listener, app).await.unwrap();
    });
    
    // Create Tray Menu and Items
    let menu = Menu::new();
    let open_item = MenuItem::new("Open Nova Suite Dashboard", true, None);
    let autostart_item = CheckMenuItem::new("Launch on Windows Startup", true, is_autostart_enabled(), None);
    let quit_item = MenuItem::new("Exit Nova Live Suite", true, None);
    
    menu.append(&open_item).unwrap();
    menu.append(&autostart_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&quit_item).unwrap();
    
    let mut tray_icon = Some(
        TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Nova Live Suite")
            .with_icon(create_tray_icon_image())
            .build()
            .unwrap(),
    );
    
    // Win32 Event loop
    let menu_channel = MenuEvent::receiver();
    
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            PeekMessageW, TranslateMessage, DispatchMessageW, MSG, PM_REMOVE
        };
        let mut msg: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            
            if let Ok(event) = menu_channel.try_recv() {
                if event.id == open_item.id() {
                    let _ = open::that(format!("http://127.0.0.1:{}", PORT));
                } else if event.id == autostart_item.id() {
                    if is_autostart_enabled() {
                        disable_autostart();
                        autostart_item.set_checked(false);
                    } else {
                        enable_autostart();
                        autostart_item.set_checked(true);
                    }
                } else if event.id == quit_item.id() {
                    let _ = tray_icon.take();
                    std::process::exit(0);
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}
