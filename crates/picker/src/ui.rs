use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, Entry, EventControllerKey, Label,
    ListBox, ListBoxRow, Orientation, ScrolledWindow,
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
    attach_receiver(receiver, &list, &status, &model, &client, &runtime);
    attach_search(&search, sender.clone(), client.clone(), runtime.clone());
    attach_selection_tracking(&list, model.clone());
    attach_row_activation(
        &list,
        &window,
        model.clone(),
        client.clone(),
        runtime.clone(),
    );
    attach_keyboard_shortcuts(&window, &list, model.clone(), client, runtime);

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
    list: &ListBox,
    status: &Label,
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
) {
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
                    render_items(&list, &model, &client, &runtime);
                }
                UiMessage::SearchFailed(error) => {
                    status.set_text(&error);
                    model.borrow_mut().replace_items(Vec::new());
                    render_items(&list, &model, &client, &runtime);
                }
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
    window: &ApplicationWindow,
    model: Rc<RefCell<PickerModel>>,
    client: IpcClient,
    runtime: Arc<Runtime>,
) {
    let window = window.clone();
    list.connect_row_activated(move |_, _| {
        restore_selected(&window, &model, &client, &runtime);
    });
}

fn attach_keyboard_shortcuts(
    window: &ApplicationWindow,
    list: &ListBox,
    model: Rc<RefCell<PickerModel>>,
    client: IpcClient,
    runtime: Arc<Runtime>,
) {
    let controller = EventControllerKey::new();
    let window_for_keys = window.clone();
    let list_for_keys = list.clone();
    controller.connect_key_pressed(move |_, key, _, state| match key {
        gdk::Key::Escape => {
            window_for_keys.close();
            glib::Propagation::Stop
        }
        gdk::Key::Return | gdk::Key::KP_Enter => {
            restore_selected(&window_for_keys, &model, &client, &runtime);
            glib::Propagation::Stop
        }
        gdk::Key::Delete => {
            delete_selected(&list_for_keys, &model, &client, &runtime);
            glib::Propagation::Stop
        }
        gdk::Key::p if state.contains(gdk::ModifierType::CONTROL_MASK) => {
            toggle_pin_selected(&list_for_keys, &model, &client, &runtime);
            glib::Propagation::Stop
        }
        gdk::Key::f if state.contains(gdk::ModifierType::CONTROL_MASK) => {
            toggle_favorite_selected(&list_for_keys, &model, &client, &runtime);
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
    window.add_controller(controller);
}

fn render_items(
    list: &ListBox,
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
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
            list,
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
    list: &ListBox,
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

    let pin_list = list.clone();
    let pin_model = model.clone();
    let pin_client = client.clone();
    let pin_runtime = runtime.clone();
    pin.connect_clicked(move |_| {
        select_item_by_id(&pin_model, item_id);
        toggle_pin_selected(&pin_list, &pin_model, &pin_client, &pin_runtime);
    });

    let favorite_list = list.clone();
    favorite.connect_clicked(move |_| {
        select_item_by_id(&model, item_id);
        toggle_favorite_selected(&favorite_list, &model, &client, &runtime);
    });

    row.set_child(Some(&container));
    row
}

fn restore_selected(
    window: &ApplicationWindow,
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
) {
    let Some(id) = model.borrow().selected_item().map(|item| item.id) else {
        return;
    };
    run_ipc(runtime, client.clone(), move |client| {
        let _ = client.restore(id, true).await;
    });
    window.close();
}

fn delete_selected(
    list: &ListBox,
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
) {
    let Some(item) = model.borrow_mut().remove_selected() else {
        return;
    };
    run_ipc(runtime, client.clone(), move |client| {
        let _ = client.delete(item.id).await;
    });
    render_items(list, model, client, runtime);
}

fn toggle_pin_selected(
    list: &ListBox,
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
) {
    let Some((id, value)) = ({
        let mut model = model.borrow_mut();
        model.toggle_selected_pin();
        model.selected_item().map(|item| (item.id, item.is_pinned))
    }) else {
        return;
    };
    run_ipc(runtime, client.clone(), move |client| {
        let _ = client.pin(id, value).await;
    });
    render_items(list, model, client, runtime);
}

fn toggle_favorite_selected(
    list: &ListBox,
    model: &Rc<RefCell<PickerModel>>,
    client: &IpcClient,
    runtime: &Arc<Runtime>,
) {
    let Some((id, value)) = ({
        let mut model = model.borrow_mut();
        model.toggle_selected_favorite();
        model
            .selected_item()
            .map(|item| (item.id, item.is_favorite))
    }) else {
        return;
    };
    run_ipc(runtime, client.clone(), move |client| {
        let _ = client.favorite(id, value).await;
    });
    render_items(list, model, client, runtime);
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

pub fn preview_text(item: &ClipboardItem) -> String {
    if let Some(text) = &item.payload.text_plain {
        return text
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(160)
            .collect();
    }
    if item.payload.text_html.is_some() {
        return "[HTML content]".to_string();
    }
    if item.payload.image_png.is_some() {
        return "[PNG image]".to_string();
    }
    "[Unsupported content]".to_string()
}

fn item_label(item: &ClipboardItem) -> Label {
    let prefix = match (item.is_pinned, item.is_favorite) {
        (true, true) => "[Pinned] [Favorite] ",
        (true, false) => "[Pinned] ",
        (false, true) => "[Favorite] ",
        (false, false) => "",
    };
    let label = Label::new(Some(&format!("{prefix}{}", preview_text(item))));
    label.set_xalign(0.0);
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
    fn preview_text_falls_back_to_content_kind() {
        let item = item_with_payload(rcopy_core::ClipboardPayload {
            text_plain: None,
            text_html: None,
            image_png: Some(vec![1, 2, 3]),
        });

        assert_eq!(preview_text(&item), "[PNG image]");
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
