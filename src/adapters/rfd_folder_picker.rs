use crate::ports::FolderPicker;
use rfd::AsyncFileDialog;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct RfdFolderPicker {
    runtime: Arc<Runtime>,
}

impl RfdFolderPicker {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }
}

impl FolderPicker for RfdFolderPicker {
    fn select_folder(
        &self,
        starting_path: Option<PathBuf>,
    ) -> tokio::sync::oneshot::Receiver<Option<PathBuf>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let rt = self.runtime.clone();

        rt.spawn(async move {
            let mut dialog = AsyncFileDialog::new().set_title("Select Download Directory");

            // Set starting directory if provided
            if let Some(start) = starting_path {
                dialog = dialog.set_directory(&start);
            }

            let result = dialog
                .pick_folder()
                .await
                .map(|handle| handle.path().to_path_buf());

            let _ = tx.send(result);
        });

        rx
    }
}
