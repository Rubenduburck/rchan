use super::post::Post;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexThread {
    pub posts: Vec<Post>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Index {
    pub threads: Vec<IndexThread>,
}
