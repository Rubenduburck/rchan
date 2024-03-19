use serde::{Deserialize, Serialize};

use super::post::Post;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogPage {
    pub page: i32,
    pub threads: Vec<Post>,
}
