pub fn remove_html(input: &str) -> Result<String, html_entities::DecodeError> {
    html_entities::decode_html_entities(input).map(|decoded| {
        // Replace <br> with newline
        let br_re = regex::Regex::new("<br\\s*/?>").unwrap();
        let decoded = br_re.replace_all(&decoded, "\n");

        // Replace &gt; with >
        let gt_re = regex::Regex::new("&gt;").unwrap();
        let converted = gt_re.replace_all(&decoded, ">");

        // Remove all html tags
        let re = regex::Regex::new("<[^>]*>").unwrap();
        re.replace_all(&converted, "").into_owned()
    })
}
