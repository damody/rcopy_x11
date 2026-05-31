use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, Entry, EventControllerKey, Label,
    ListBox, ListBoxRow, Orientation, PropagationPhase, ScrolledWindow,
};
use libadwaita as adw;
use rcopy_core::ClipboardItem;
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::ipc_client::IpcClient;
use crate::view_model::PickerModel;

enum UiMessage {
    SearchFinished(Vec<ClipboardItem>),
    SearchFailed(String),
    RestoreFinished(Result<crate::ipc_client::RestoreResponse, String>),
    DeleteFinished {
        id: Uuid,
        result: Result<(), String>,
    },
    PinFinished {
        id: Uuid,
        value: bool,
        result: Result<(), String>,
    },
    FavoriteFinished {
        id: Uuid,
        value: bool,
        result: Result<(), String>,
    },
}

pub fn run() {
    adw::init().expect("libadwaita initializes");
    let app = Application::builder()
        .application_id("dev.rcopy.picker")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let runtime = match Runtime::new() {
        Ok(runtime) => Arc::new(runtime),
        Err(error) => {
            eprintln!("failed to start async runtime: {error}");
            return;
        }
    };
    let client = IpcClient::default_socket();
    let model = Rc::new(RefCell::new(PickerModel::new(Vec::new())));

    let search = Entry::builder()
        .placeholder_text("Search clipboard")
        .hexpand(true)
        .build();
    let status = Label::new(None);
    status.set_xalign(0.0);

    let list = ListBox::builder().vexpand(true).build();
    list.set_activate_on_single_click(false);
    let scroller = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .child(&list)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 8);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);
    root.append(&search);
    root.append(&scroller);
    root.append(&status);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("rcopy")
        .default_width(720)
        .default_height(520)
        .child(&root)
        .build();

    let (sender, receiver) = std::sync::mpsc::channel();
    attach_receiver(
        receiver,
        &window,
        &list,
        &status,
        &model,
        &client,
        &runtime,
        sender.clone(),
    );
    attach_search(&search, sender.clone(), client.clone(), runtime.clone());
    attach_selection_tracking(&list, model.clone());
    attach_row_activation(
        &list,
        &window,
        model.clone(),
        client.clone(),
        runtime.clone(),
        sender.clone(),
    );
    attach_keyboard_shortcuts(
        &window,
        &search,
        &list,
        model.clone(),
        client,
        runtime,
        sender,
    );

    window.present();
    search.grab_focus();
}

fn attach_search(
    search: &Entry,
    sender: Sender<UiMessage>,
    client: IpcClient,
    runtime: Arc<Runtime>,
) {
    let search_sender = sender.clone();
    let search_client = client.clone();
    let search_runtime = runtime.clone();
    search.connect_changed(move |entry| {
        attach_search_request(
            search_sender.clone(),
            search_client.clone(),
            search_runtime.clone(),
            entry.text().as_ref(),
        );
    });

    attach_search_request(sender, client, runtime, "");
}

fn attach_search_request(
    sender: Sender<UiMessage>,
    client: IpcClient,
    runtime: Arc<Runtime>,
    query: &str,
) {
    let query = query.to_string();
    std::thread::spawn(move || {
        let message = match runtime.block_on(client.search(&query)) {
            Ok(items) => UiMessage::SearchFinished(items),
            Err(error) => UiMessage::SearchFailed(error.to_string()),
        };
        let _ = sender.send(message);
    });
}

fn attach_receiver(
    receiver: Receiver<UiMessage>,
    window: &ApplicationWindow,
    list: &ListBox,
    status: &Label,
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
    sender: Sender<UiMessage>,
) {
    let window = window.clone();
    let list = list.clone();
    let status = status.clone();
    let model = model.clone();
    let client = client.clone();
    let runtime = runtime.clone();
    glib::timeout_add_local(Duration::from_millis(50), move || {
        for message in receiver.try_iter() {
            match message {
                UiMessage::SearchFinished(items) => {
                    status.set_text("");
                    model.borrow_mut().replace_items(items);
                    render_items(&list, &model, &client, &runtime, &sender);
                }
                UiMessage::SearchFailed(error) => {
                    status.set_text(&error);
                    model.borrow_mut().replace_items(Vec::new());
                    render_items(&list, &model, &client, &runtime, &sender);
                }
                UiMessage::RestoreFinished(Ok(response)) => {
                    if response.paste_attempted && !response.paste_succeeded {
                        status.set_text("Clipboard restored; automatic paste did not complete");
                    }
                    window.close();
                }
                UiMessage::RestoreFinished(Err(error)) => {
                    status.set_text(&format!("Restore failed: {error}"));
                }
                UiMessage::DeleteFinished { id, result } => match result {
                    Ok(()) => {
                        model.borrow_mut().remove_by_id(id);
                        status.set_text("");
                        render_items(&list, &model, &client, &runtime, &sender);
                    }
                    Err(error) => status.set_text(&format!("Delete failed: {error}")),
                },
                UiMessage::PinFinished { id, value, result } => match result {
                    Ok(()) => {
                        model.borrow_mut().set_pin_by_id(id, value);
                        status.set_text("");
                        render_items(&list, &model, &client, &runtime, &sender);
                    }
                    Err(error) => status.set_text(&format!("Pin failed: {error}")),
                },
                UiMessage::FavoriteFinished { id, value, result } => match result {
                    Ok(()) => {
                        model.borrow_mut().set_favorite_by_id(id, value);
                        status.set_text("");
                        render_items(&list, &model, &client, &runtime, &sender);
                    }
                    Err(error) => status.set_text(&format!("Favorite failed: {error}")),
                },
            }
        }
        glib::ControlFlow::Continue
    });
}

fn attach_selection_tracking(list: &ListBox, model: Rc<RefCell<PickerModel>>) {
    list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            model.borrow_mut().select(row.index() as usize);
        }
    });
}

fn attach_row_activation(
    list: &ListBox,
    _window: &ApplicationWindow,
    model: Rc<RefCell<PickerModel>>,
    client: IpcClient,
    runtime: Arc<Runtime>,
    sender: Sender<UiMessage>,
) {
    let sender = sender.clone();
    list.connect_row_activated(move |_, _| {
        restore_selected(&model, &client, &runtime, &sender, false);
    });
}

fn attach_keyboard_shortcuts(
    window: &ApplicationWindow,
    search: &Entry,
    list: &ListBox,
    model: Rc<RefCell<PickerModel>>,
    client: IpcClient,
    runtime: Arc<Runtime>,
    sender: Sender<UiMessage>,
) {
    let window_controller = keyboard_controller(
        window,
        list,
        model.clone(),
        client.clone(),
        runtime.clone(),
        sender.clone(),
    );
    window.add_controller(window_controller);

    let search_controller = keyboard_controller(window, list, model, client, runtime, sender);
    search_controller.set_propagation_phase(PropagationPhase::Capture);
    search.add_controller(search_controller);
}

fn keyboard_controller(
    window: &ApplicationWindow,
    list: &ListBox,
    model: Rc<RefCell<PickerModel>>,
    client: IpcClient,
    runtime: Arc<Runtime>,
    sender: Sender<UiMessage>,
) -> EventControllerKey {
    let controller = EventControllerKey::new();
    let window_for_keys = window.clone();
    let list_for_keys = list.clone();
    controller.connect_key_pressed(move |_, key, _, state| match key {
        gdk::Key::Escape => {
            window_for_keys.close();
            glib::Propagation::Stop
        }
        gdk::Key::Up => {
            select_previous(&list_for_keys, &model);
            glib::Propagation::Stop
        }
        gdk::Key::Down => {
            select_next(&list_for_keys, &model);
            glib::Propagation::Stop
        }
        gdk::Key::Return | gdk::Key::KP_Enter => {
            let plain_text_only = state.contains(gdk::ModifierType::SHIFT_MASK);
            restore_selected(&model, &client, &runtime, &sender, plain_text_only);
            glib::Propagation::Stop
        }
        gdk::Key::Delete => {
            delete_selected(&model, &client, &runtime, &sender);
            glib::Propagation::Stop
        }
        gdk::Key::p if state.contains(gdk::ModifierType::CONTROL_MASK) => {
            toggle_pin_selected(&model, &client, &runtime, &sender);
            glib::Propagation::Stop
        }
        gdk::Key::f if state.contains(gdk::ModifierType::CONTROL_MASK) => {
            toggle_favorite_selected(&model, &client, &runtime, &sender);
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
    controller
}

fn render_items(
    list: &ListBox,
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
    sender: &Sender<UiMessage>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let items = model.borrow().items.clone();
    if items.is_empty() {
        let row = ListBoxRow::new();
        let label = Label::new(Some("No clipboard items"));
        label.set_xalign(0.0);
        row.set_child(Some(&label));
        row.set_selectable(false);
        list.append(&row);
        return;
    }

    for item in items {
        list.append(&item_row(
            item,
            model.clone(),
            client.clone(),
            runtime.clone(),
            sender.clone(),
        ));
    }

    if let Some(row) = list.row_at_index(model.borrow().selected_index as i32) {
        list.select_row(Some(&row));
    }
}

fn item_row(
    item: ClipboardItem,
    model: Rc<RefCell<PickerModel>>,
    client: IpcClient,
    runtime: Arc<Runtime>,
    sender: Sender<UiMessage>,
) -> ListBoxRow {
    let row = ListBoxRow::new();
    let container = GtkBox::new(Orientation::Horizontal, 8);
    container.set_margin_top(8);
    container.set_margin_bottom(8);
    container.set_margin_start(8);
    container.set_margin_end(8);

    let label = item_label(&item);
    label.set_hexpand(true);
    container.append(&label);

    let item_id = item.id;
    let pin = Button::with_label(if item.is_pinned { "Unpin" } else { "Pin" });
    let favorite = Button::with_label(if item.is_favorite {
        "Unfavorite"
    } else {
        "Favorite"
    });
    container.append(&pin);
    container.append(&favorite);

    let pin_model = model.clone();
    let pin_client = client.clone();
    let pin_runtime = runtime.clone();
    let pin_sender = sender.clone();
    pin.connect_clicked(move |_| {
        select_item_by_id(&pin_model, item_id);
        toggle_pin_selected(&pin_model, &pin_client, &pin_runtime, &pin_sender);
    });

    let favorite_sender = sender.clone();
    favorite.connect_clicked(move |_| {
        select_item_by_id(&model, item_id);
        toggle_favorite_selected(&model, &client, &runtime, &favorite_sender);
    });

    row.set_child(Some(&container));
    row
}

fn restore_selected(
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
    sender: &Sender<UiMessage>,
    plain_text_only: bool,
) {
    let Some(id) = model.borrow().selected_item().map(|item| item.id) else {
        return;
    };
    let sender = sender.clone();
    run_ipc(runtime, client.clone(), move |client| async move {
        let result = client
            .restore(id, true, plain_text_only)
            .await
            .map_err(|error| error.to_string());
        let _ = sender.send(UiMessage::RestoreFinished(result));
    });
}

fn delete_selected(
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
    sender: &Sender<UiMessage>,
) {
    let Some(id) = model.borrow().selected_item().map(|item| item.id) else {
        return;
    };
    let sender = sender.clone();
    run_ipc(runtime, client.clone(), move |client| async move {
        let result = client.delete(id).await.map_err(|error| error.to_string());
        let _ = sender.send(UiMessage::DeleteFinished { id, result });
    });
}

fn toggle_pin_selected(
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
    sender: &Sender<UiMessage>,
) {
    let Some((id, value)) = model
        .borrow()
        .selected_item()
        .map(|item| (item.id, !item.is_pinned))
    else {
        return;
    };
    let sender = sender.clone();
    run_ipc(runtime, client.clone(), move |client| async move {
        let result = client
            .pin(id, value)
            .await
            .map_err(|error| error.to_string());
        let _ = sender.send(UiMessage::PinFinished { id, value, result });
    });
}

fn toggle_favorite_selected(
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
    sender: &Sender<UiMessage>,
) {
    let Some((id, value)) = model
        .borrow()
        .selected_item()
        .map(|item| (item.id, !item.is_favorite))
    else {
        return;
    };
    let sender = sender.clone();
    run_ipc(runtime, client.clone(), move |client| async move {
        let result = client
            .favorite(id, value)
            .await
            .map_err(|error| error.to_string());
        let _ = sender.send(UiMessage::FavoriteFinished { id, value, result });
    });
}

fn run_ipc<F, Fut>(runtime: &Arc<Runtime>, client: IpcClient, action: F)
where
    F: FnOnce(IpcClient) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let handle = runtime.handle().clone();
    std::thread::spawn(move || {
        handle.block_on(action(client));
    });
}

fn select_item_by_id(model: &Rc<RefCell<PickerModel>>, id: Uuid) {
    let index = model
        .borrow()
        .items
        .iter()
        .position(|item| item.id == id)
        .unwrap_or(0);
    model.borrow_mut().select(index);
}

fn select_previous(list: &ListBox, model: &Rc<RefCell<PickerModel>>) {
    model.borrow_mut().move_up();
    select_model_row(list, model);
}

fn select_next(list: &ListBox, model: &Rc<RefCell<PickerModel>>) {
    model.borrow_mut().move_down();
    select_model_row(list, model);
}

fn select_model_row(list: &ListBox, model: &Rc<RefCell<PickerModel>>) {
    if let Some(row) = list.row_at_index(model.borrow().selected_index as i32) {
        list.select_row(Some(&row));
    }
}

pub fn preview_text(item: &ClipboardItem) -> String {
    if let Some(text) = &item.payload.text_plain {
        return condense_text(text);
    }
    if let Some(html) = &item.payload.text_html {
        let text = condense_text(&strip_html_tags(html));
        if text.is_empty() {
            return "[HTML content]".to_string();
        }
        return text;
    }
    if let Some(image) = &item.payload.image_png {
        return match png_dimensions(image) {
            Some((width, height)) => format!("PNG image, {width}x{height}, {} bytes", image.len()),
            None => format!("PNG image, {} bytes", image.len()),
        };
    }
    "[Unsupported content]".to_string()
}

fn content_kind(item: &ClipboardItem) -> &'static str {
    if item.payload.image_png.is_some() {
        "PNG"
    } else if item.payload.text_html.is_some() {
        "HTML"
    } else if item.payload.text_plain.is_some() {
        "Text"
    } else {
        "Unsupported"
    }
}

fn condensed_flags(item: &ClipboardItem) -> String {
    format!(
        "Pinned: {} | Favorite: {}",
        if item.is_pinned { "yes" } else { "no" },
        if item.is_favorite { "yes" } else { "no" }
    )
}

fn condense_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect()
}

fn strip_html_tags(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for character in html.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    text
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 24 || &bytes[0..8] != PNG_SIGNATURE || &bytes[12..16] != b"IHDR" {
        return None;
    }

    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

fn item_label(item: &ClipboardItem) -> Label {
    let label = Label::new(Some(&format!(
        "{} | Created: {} | {}\n{}",
        content_kind(item),
        item.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
        condensed_flags(item),
        preview_text(item)
    )));
    label.set_xalign(0.0);
    label.set_wrap(true);
    label
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_text_prefers_plain_text() {
        let item = item_with_payload(rcopy_core::ClipboardPayload {
            text_plain: Some("alpha\nbeta".into()),
            text_html: Some("<b>alpha</b>".into()),
            image_png: None,
        });

        assert_eq!(preview_text(&item), "alpha");
    }

    #[test]
    fn preview_text_condenses_html_text() {
        let item = item_with_payload(rcopy_core::ClipboardPayload {
            text_plain: None,
            text_html: Some("<p>alpha <strong>beta</strong></p>".into()),
            image_png: None,
        });

        assert_eq!(preview_text(&item), "alpha beta");
    }

    #[test]
    fn preview_text_falls_back_to_png_metadata() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&640u32.to_be_bytes());
        png.extend_from_slice(&480u32.to_be_bytes());

        let item = item_with_payload(rcopy_core::ClipboardPayload {
            text_plain: None,
            text_html: None,
            image_png: Some(png),
        });

        assert_eq!(preview_text(&item), "PNG image, 640x480, 24 bytes");
    }

    fn item_with_payload(payload: rcopy_core::ClipboardPayload) -> ClipboardItem {
        let now = chrono::Utc::now();
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
