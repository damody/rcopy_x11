use crate::ClipboardItem;

pub fn rank_items(query: &str, mut items: Vec<ClipboardItem>) -> Vec<ClipboardItem> {
    let needle = query.trim().to_lowercase();
    items.retain(|item| {
        item.deleted_at.is_none()
            && (needle.is_empty()
                || item
                    .payload
                    .text_plain
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&needle)
                || item
                    .payload
                    .text_html
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&needle))
    });
    items.sort_by(|a, b| {
        b.is_pinned
            .cmp(&a.is_pinned)
            .then_with(|| b.is_favorite.cmp(&a.is_favorite))
            .then_with(|| b.last_used_at.cmp(&a.last_used_at))
    });
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::sample_item;

    #[test]
    fn search_prefers_pinned_then_favorite_then_recent_matches() {
        let recent = sample_item("recent alpha", false, false, 30);
        let favorite = sample_item("favorite alpha", false, true, 20);
        let pinned = sample_item("pinned alpha", true, false, 10);
        let ranked = rank_items(
            "alpha",
            vec![recent.clone(), favorite.clone(), pinned.clone()],
        );
        assert_eq!(ranked[0].id, pinned.id);
        assert_eq!(ranked[1].id, favorite.id);
        assert_eq!(ranked[2].id, recent.id);
    }
}
