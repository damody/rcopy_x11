#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MimeKind {
    TextPlain,
    TextHtml,
    ImagePng,
}

pub fn select_supported_mimes(offered: &[String]) -> Vec<MimeKind> {
    let has_text = offered.iter().any(|mime| mime.starts_with("text/plain"));
    let has_html = offered.iter().any(|mime| mime == "text/html");
    let has_png = offered.iter().any(|mime| mime == "image/png");
    let mut selected = Vec::new();
    if has_text {
        selected.push(MimeKind::TextPlain);
    }
    if has_html {
        selected.push(MimeKind::TextHtml);
    }
    if has_png {
        selected.push(MimeKind::ImagePng);
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_mime_types_are_selected_in_stable_order() {
        let offered = vec![
            "image/jpeg".to_string(),
            "text/html".to_string(),
            "text/plain;charset=utf-8".to_string(),
            "image/png".to_string(),
        ];
        assert_eq!(
            select_supported_mimes(&offered),
            vec![MimeKind::TextPlain, MimeKind::TextHtml, MimeKind::ImagePng]
        );
    }
}
