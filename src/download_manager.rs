use crate::database::Database;
use crate::utils::string_utils::{sanitize_file_name, sanitize_folder_uri};
use bytes::Bytes;
use egui::TextBuffer;
use reqwest::blocking::Client;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct DownloadManager {
    database: Database,
    client: Client,
}

impl DownloadManager {
    pub fn new(database: Database) -> Self {
        Self {
            database,
            client: Client::new(),
        }
    }

    pub fn file_exists(&self, folders: Vec<String>, file_name: &str) -> bool {
        let download_path = self.database.get_download_directory().unwrap();
        let download_dir = construct_download_path(download_path, folders);

        if !download_dir.exists() {
            return false;
        }

        let entries = match fs::read_dir(&download_dir) {
            Ok(e) => e,
            Err(_) => return false,
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(&sanitize_file_name(&*file_name)) {
                return true;
            }
        }

        false
    }

    pub fn download(
        &self,
        url: String,
        folders: Vec<String>,
        file_name: String,
    ) -> Result<(), String> {
        // Get Download Path
        let download_path = self.database.get_download_directory().unwrap();
        let mut download_dir = construct_download_path(download_path, folders);

        if !download_dir.exists() {
            fs::create_dir_all(&download_dir).unwrap();
        }

        let response = self.client.get(url).send();

        if response.is_err() {
            return Err("Failed to download file".to_string());
        }

        let response_result = response.unwrap();

        let headers = response_result.headers();

        let ext = headers
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                // looks for filename="something.ext" or filename=something.ext
                v.split(';')
                    .map(str::trim)
                    .find(|p| p.to_lowercase().starts_with("filename="))
                    .and_then(|p| p.splitn(2, '=').nth(1))
                    .map(|f| f.trim_matches('"').to_string())
            })
            .and_then(|filename| {
                Path::new(&filename)
                    .extension()
                    .map(|e| format!(".{}", e.to_string_lossy()))
            })
            // 2. Try Content-Type
            .or_else(|| {
                headers
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|mime| mime_to_ext(mime))
                    .map(|e| e.to_string())
            })
            // 3. Fallback
            .unwrap_or_else(|| ".bin".to_string());

        download_dir = download_dir.join(format!("{}{}", sanitize_file_name(&*file_name), ext));

        let bytes: Bytes = response_result.bytes().unwrap();
        let file_write = fs::write(&download_dir, &bytes);
        if file_write.is_err() {
            println!("{}", download_dir.to_str().unwrap());
            println!("Error decoding json: {:?}", file_write.err().unwrap());
            return Err("Failed to write file".to_string());
        }

        Ok(())
    }
}

fn construct_download_path(base_path: String, folders: Vec<String>) -> PathBuf {
    let mut download_dir = Path::new(&base_path).to_path_buf();
    for folder in folders {
        download_dir = download_dir.join(sanitize_folder_uri(&*folder));
    }
    download_dir
}
fn mime_to_ext(mime: &str) -> Option<&'static str> {
    // Strip any parameters e.g. "text/html; charset=utf-8" -> "text/html"
    let mime = mime.split(';').next()?.trim();
    match mime {
        "audio/mpeg" => Some(".mp3"),
        "audio/ogg" => Some(".ogg"),
        "audio/wav" => Some(".wav"),
        "audio/flac" => Some(".flac"),
        "video/mp4" => Some(".mp4"),
        "video/webm" => Some(".webm"),
        "image/jpeg" => Some(".jpg"),
        "image/png" => Some(".png"),
        "image/gif" => Some(".gif"),
        "application/pdf" => Some(".pdf"),
        "application/zip" => Some(".zip"),
        "text/plain" => Some(".txt"),
        "text/html" => Some(".html"),
        _ => None,
    }
}
