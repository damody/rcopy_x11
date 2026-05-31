use rcopy_core::ClipboardItem;

#[derive(Clone, Debug)]
pub struct PickerModel {
    pub items: Vec<ClipboardItem>,
    pub selected_index: usize,
}

impl PickerModel {
    pub fn new(items: Vec<ClipboardItem>) -> Self {
        Self {
            items,
            selected_index: 0,
        }
    }

    pub fn replace_items(&mut self, items: Vec<ClipboardItem>) {
        self.items = items;
        self.selected_index = 0;
    }

    pub fn select(&mut self, index: usize) {
        if !self.items.is_empty() {
            self.selected_index = index.min(self.items.len() - 1);
        }
    }

    pub fn move_down(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.items.len() - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn selected_item(&self) -> Option<&ClipboardItem> {
        self.items.get(self.selected_index)
    }

    pub fn remove_selected(&mut self) -> Option<ClipboardItem> {
        if self.items.is_empty() {
            return None;
        }

        let removed = self.items.remove(self.selected_index);
        if self.selected_index >= self.items.len() {
            self.selected_index = self.items.len().saturating_sub(1);
        }
        Some(removed)
    }

    pub fn toggle_selected_pin(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected_index) {
            item.is_pinned = !item.is_pinned;
        }
    }

    pub fn toggle_selected_favorite(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected_index) {
            item.is_favorite = !item.is_favorite;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_index_moves_within_bounds() {
        let mut model = PickerModel::new(vec![item("a"), item("b")]);
        model.move_down();
        model.move_down();
        assert_eq!(model.selected_index, 1);
        model.move_up();
        model.move_up();
        assert_eq!(model.selected_index, 0);
    }

    #[test]
    fn selected_item_returns_current_item() {
        let mut model = PickerModel::new(vec![item("a"), item("b")]);
        model.move_down();
        assert_eq!(
            model.selected_item().unwrap().payload.text_plain.as_deref(),
            Some("b")
        );
    }

    #[test]
    fn remove_selected_item_updates_selection() {
        let mut model = PickerModel::new(vec![item("a"), item("b")]);
        model.move_down();
        let removed = model.remove_selected().unwrap();
        assert_eq!(removed.payload.text_plain.as_deref(), Some("b"));
        assert_eq!(model.selected_index, 0);
        assert_eq!(model.items.len(), 1);
    }

    #[test]
    fn toggle_selected_pin_flips_state() {
        let mut model = PickerModel::new(vec![item("a")]);
        model.toggle_selected_pin();
        assert!(model.selected_item().unwrap().is_pinned);
    }

    #[test]
    fn toggle_selected_favorite_flips_state() {
        let mut model = PickerModel::new(vec![item("a")]);
        model.toggle_selected_favorite();
        assert!(model.selected_item().unwrap().is_favorite);
    }

    fn item(text: &str) -> ClipboardItem {
        let now = chrono::Utc::now();
        let payload = rcopy_core::ClipboardPayload {
            text_plain: Some(text.to_string()),
            text_html: None,
            image_png: None,
        };
        ClipboardItem {
            id: uuid::Uuid::new_v4(),
            created_at: now,
            last_used_at: now,
            content_hash: rcopy_core::content_hash(&payload),
            mime_types: vec!["text/plain".into()],
            payload,
            is_pinned: false,
            is_favorite: false,
            deleted_at: None,
        }
    }
}
