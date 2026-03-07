use std::path::PathBuf;
use tokio::sync::oneshot;

pub trait FolderPicker: Send + Sync {
    fn select_folder(&self, starting_path: Option<PathBuf>) -> oneshot::Receiver<Option<PathBuf>>;
}
