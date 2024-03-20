use std::fmt::{Display, Formatter};


#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Endpoint {
    Boards,
    Threads(String),
    Catalog(String),
    Archive(String),
    Thread(String, i32),
    Index(String, i32),
}

impl Endpoint {
    const BASE_URL: &'static str = "a.4cdn.org";

    pub fn http(&self) -> String {
        format!("http://{}", self)
    }

    pub fn https(&self) -> String {
        format!("https://{}", self)
    }

    pub fn url(&self, https: bool) -> String {
        if https {
            self.https()
        } else {
            self.http()
        }
    }

}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Boards => format!("{}/boards.json", Self::BASE_URL),
                Self::Threads(board) => format!("{}/{}/threads.json", Self::BASE_URL, board),
                Self::Catalog(board) => format!("{}/{}/catalog.json", Self::BASE_URL, board),
                Self::Archive(board) => format!("{}/{}/archive.json", Self::BASE_URL, board),
                Self::Index(board, page) => format!("{}/{}/{}.json", Self::BASE_URL, board, page),
                Self::Thread(board, thread_no) => {
                    format!("{}/{}/thread/{}.json", Self::BASE_URL, board, thread_no)
                }
            }
        )
    }
}
