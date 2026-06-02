pub mod clipboard;
pub mod paste;

pub use clipboard::{ClipboardBackend, ClipboardError, MemoryClipboard, X11Clipboard};
pub use paste::{DisabledPasteBackend, PasteBackend, PasteError, XdotoolPasteBackend};
