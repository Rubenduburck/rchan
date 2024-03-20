use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thread {
    pub posts: Vec<Post>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreadPage {
    pub page: i32,
    pub threads: Vec<Post>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Post {
    pub no: i32,
    pub sticky: Option<i32>,
    pub closed: Option<i32>,
    pub now: Option<String>,
    pub name: Option<String>,
    pub sub: Option<String>,
    pub com: Option<String>,
    pub filename: Option<String>,
    pub ext: Option<String>,
    pub w: Option<i32>,
    pub h: Option<i32>,
    pub tn_w: Option<i32>,
    pub tn_h: Option<i32>,
    pub tim: Option<i64>,
    pub time: Option<i64>,
    pub md5: Option<String>,
    pub fsize: Option<i32>,
    pub resto: Option<i32>,
    pub capcode: Option<String>,
    pub semantic_url: Option<String>,
    pub replies: Option<i32>,
    pub images: Option<i32>,
    pub unique_ips: Option<i32>,
    pub omitted_posts: Option<i32>,
    pub omitted_images: Option<i32>,
    pub last_replies: Option<Vec<Post>>,
    pub last_modified: Option<i64>,
}

impl Post {
    pub fn is_op(&self) -> bool {
        self.resto.is_none()
    }
}

impl PartialEq for Post {
    fn eq(&self, other: &Self) -> bool {
        self.no == other.no
    }
}

impl Eq for Post {}

impl PartialOrd for Post {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.no.cmp(&other.no))
    }
}

impl Ord for Post {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.no.cmp(&other.no)
    }
}
