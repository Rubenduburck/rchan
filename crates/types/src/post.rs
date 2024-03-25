use regex::Regex;
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
        self.resto.is_none() || self.resto.unwrap() == 0
    }

    pub fn is_reply(&self) -> bool {
        self.resto.is_some() || self.resto.unwrap() != 0
    }

    pub fn thread_no(&self) -> i32 {
        match self.resto {
            Some(resto) if resto != 0 => resto,
            _ => self.no,
        }
    }

    pub fn post_no(&self) -> i32 {
        self.no
    }

    pub fn is_sticky(&self) -> bool {
        self.sticky.is_some()
    }

    pub fn is_closed(&self) -> bool {
        self.closed.is_some()
    }

    pub fn has_image(&self) -> bool {
        self.tim.is_some()
    }

    pub fn has_replies(&self) -> bool {
        self.replies.is_some()
    }

    pub fn clean_comment(&self) -> Option<String> {
        self.com
            .as_ref()
            .map(|c| crate::utils::remove_html(c).unwrap())
    }

    pub fn quotes(&self) -> Vec<i32> {
        let re = Regex::new(r###"<a href="#p(\d+)" class="quotelink">&gt;&gt;\d+</a>"###).unwrap();
        let mut quotes = Vec::new();
        if let Some(comment) = &self.com {
            for cap in re.captures_iter(comment) {
                quotes.push(cap[1].parse().unwrap());
            }
        }
        quotes
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

#[cfg(test)]
mod tests {
    use super::*;

    fn post_from_comment(comment: &str) -> Post {
        Post {
            no: 1,
            sticky: None,
            closed: None,
            now: None,
            name: None,
            sub: None,
            com: Some(comment.to_string()),
            filename: None,
            ext: None,
            w: None,
            h: None,
            tn_w: None,
            tn_h: None,
            tim: None,
            time: None,
            md5: None,
            fsize: None,
            resto: None,
            capcode: None,
            semantic_url: None,
            replies: None,
            images: None,
            unique_ips: None,
            omitted_posts: None,
            omitted_images: None,
            last_replies: None,
            last_modified: None,
        }
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_clean_up_4chan_post() {
        let comment = "<span class=\"quote\">&gt;be me\n&gt;be anon\n&gt;be greentexting</span>";
        let post = post_from_comment(comment);
        let expected = ">be me\n>be anon\n>be greentexting";
        assert_eq!(post.clean_comment().unwrap(), expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_clean_up_4chan_post_2() {
        let comment =  "<a href=\"#p99602349\" class=\"quotelink\">&gt;&gt;99602349</a><br><span class=\"quote\">&gt;t. cohee</span><br><pre class=\"prettyprint\">#Example 2: calculate a factorial of 5.<br>/setvar key=inp ut 5 |<br>/setvar key=i 1 |<br>/setvar key=product 1 |<br>/while left=i right=input rule=lte &quot;/mul product i \\| /setvar key=product \\| /addvar key=i 1&quot; |<br>/getvar product |<br>/echo Factorial of {{getvar::input}}: {{ pipe}} |<br>/flushvar input |<br>/flushvar i |<br>/flushvar product</pre>";
        let post = post_from_comment(comment);
        let expected = ">>99602349\n>t. cohee\n#Example 2: calculate a factorial of 5.\n/setvar key=inp ut 5 |\n/setvar key=i 1 |\n/setvar key=product 1 |\n/while left=i right=input rule=lte \"/mul product i \\| /setvar key=product \\| /addvar key=i 1\" |\n/getvar product |\n/echo Factorial of {{getvar::input}}: {{ pipe}} |\n/flushvar input |\n/flushvar i |\n/flushvar product";
        assert_eq!(post.clean_comment().unwrap(), expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_quotes() {
        let comment = "<a href=\"#p99602349\" class=\"quotelink\">&gt;&gt;99602349</a><br><span class=\"quote\">&gt;t. cohee</span><br><pre class=\"prettyprint\">#Example 2: calculate a factorial of 5.<br>/setvar key=inp ut 5 |<br>/setvar key=i 1 |<br>/setvar key=product 1 |<br>/while left=i right=input rule=lte &quot;/mul product i \\| /setvar key=product \\| /addvar key=i 1&quot; |<br>/getvar product |<br>/echo Factorial of {{getvar::input}}: {{ pipe}} |<br>/flushvar input |<br>/flushvar i |<br>/flushvar product</pre>";
        let post = post_from_comment(comment);
        assert_eq!(post.quotes(), vec![99602349]);
    }
}
