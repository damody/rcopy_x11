pub mod clipboard;
pub mod paste;

pub use clipboard::{ClipboardBackend, ClipboardError, MemoryClipboard, WlClipboard};
pub use paste::{DisabledPasteBackend, PasteBackend, PasteError, WtypePasteBackend};
