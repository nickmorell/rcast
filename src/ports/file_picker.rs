use std::path::PathBuf;
use tokio::sync::oneshot;

/// Port abstraction for native file open/save dialogs.
/// Decouples domain logic from rfd (or any other dialog implementation).
pub trait FilePicker: Send + Sync {
    /// Opens a file-open dialog filtered to .opml and .xml files.
    /// Returns `Some(PathBuf)` on selection, `None` on cancel.
    fn pick_file(&self, title: &str) -> oneshot::Receiver<Option<PathBuf>>;

    /// Opens a file-save dialog with a suggested filename.
    /// Returns `Some(PathBuf)` on confirmation, `None` on cancel.
    fn save_file(&self, title: &str, suggested_name: &str) -> oneshot::Receiver<Option<PathBuf>>;
}
