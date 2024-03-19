use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Board {
    pub board: String,
    pub title: String,
    pub ws_board: i32,
    pub per_page: i32,
    pub pages: i32,
    pub max_filesize: i32,
    pub max_webm_filesize: i32,
    pub max_comment_chars: i32,
    pub max_webm_duration: i32,
    pub bump_limit: i32,
    pub image_limit: i32,
    pub cooldowns: Cooldowns,
    pub meta_description: String,
    pub spoilers: Option<i32>,
    pub custom_spoilers: Option<i32>,
    pub is_archived: Option<i32>,
    pub forced_anon: Option<i32>,
    pub board_flags: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cooldowns {
    pub threads: i32,
    pub replies: i32,
    pub images: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardsResponse {
    pub boards: Vec<Board>,
}
