#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MimeKind {
    TextPlain,
    TextHtml,
    ImagePng,
}

pub fn select_supported_mimes(offered: &[String]) -> Vec<MimeKind> {
    let normalized = offered
        .iter()
        .map(|mime| normalize_mime(mime))
        .collect::<Vec<_>>();
    let has_text = normalized.iter().any(|mime| mime == "text/plain");
    let has_html = normalized.iter().any(|mime| mime == "text/html");
    let has_png = normalized.iter().any(|mime| mime == "image/png");
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

fn normalize_mime(mime: &str) -> String {
    let base = mime
        .split_once(';')
        .map_or(mime, |(base, _parameters)| base)
        .trim();
    base.to_ascii_lowercase()
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

    #[test]
    fn mime_matching_normalizes_case_whitespace_and_parameters() {
        let offered = vec![
            " TEXT/PLAIN ; charset=utf-8".to_string(),
            "Text/Html; Charset=UTF-8".to_string(),
            " IMAGE/PNG ".to_string(),
        ];

        assert_eq!(
            select_supported_mimes(&offered),
            vec![MimeKind::TextPlain, MimeKind::TextHtml, MimeKind::ImagePng]
        );
    }

    #[test]
    fn mime_matching_rejects_prefix_only_matches() {
        let offered = vec!["text/plainfoo".to_string()];

        assert_eq!(select_supported_mimes(&offered), Vec::new());
    }
}
