use std::path::PathBuf;
use tokio::sync::oneshot;

/// Port abstraction for native folder selection dialogs.
/// Decouples domain logic from rfd (or any other dialog implementation).
pub trait FolderPicker: Send + Sync {
    /// Initiates folder selection asynchronously.
    /// Returns a receiver that yields:
    /// - `Some(PathBuf)` if user selected a folder
    /// - `None` if user cancelled
    /// - Channel closes if operation failed
    fn select_folder(&self, starting_path: Option<PathBuf>) -> oneshot::Receiver<Option<PathBuf>>;
}
