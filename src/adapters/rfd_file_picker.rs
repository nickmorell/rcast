use crate::ports::FilePicker;
use rfd::AsyncFileDialog;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

pub struct RfdFilePicker {
    runtime: Arc<Runtime>,
}

impl RfdFilePicker {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }
}

impl FilePicker for RfdFilePicker {
    fn pick_file(&self, title: &str) -> oneshot::Receiver<Option<PathBuf>> {
        let (tx, rx) = oneshot::channel();
        let title = title.to_string();

        self.runtime.spawn(async move {
            let result = AsyncFileDialog::new()
                .set_title(&title)
                .add_filter("OPML / XML", &["opml", "xml"])
                .add_filter("All Files", &["*"])
                .pick_file()
                .await
                .map(|h| h.path().to_path_buf());

            let _ = tx.send(result);
        });

        rx
    }

    fn save_file(&self, title: &str, suggested_name: &str) -> oneshot::Receiver<Option<PathBuf>> {
        let (tx, rx) = oneshot::channel();
        let title = title.to_string();
        let suggested_name = suggested_name.to_string();

        self.runtime.spawn(async move {
            let result = AsyncFileDialog::new()
                .set_title(&title)
                .set_file_name(&suggested_name)
                .add_filter("OPML", &["opml"])
                .save_file()
                .await
                .map(|h| h.path().to_path_buf());

            let _ = tx.send(result);
        });

        rx
    }
}
