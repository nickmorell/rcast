use crate::db::Database;
use crate::utils::string_utils::{sanitize_file_name, sanitize_folder_uri};
use bytes::Bytes;
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
        self.find_file(folders, file_name).is_some()
    }

    /// Returns the full path of a downloaded file if it exists, or `None`.
    pub fn find_file(&self, folders: Vec<String>, file_name: &str) -> Option<std::path::PathBuf> {
        let download_path = self.database.get_download_directory_sync().ok()?;
        let download_dir = construct_download_path(download_path, folders);

        let entries = fs::read_dir(&download_dir).ok()?;
        let sanitized = sanitize_file_name(file_name);

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(sanitized.as_str()) {
                return Some(entry.path());
            }
        }

        None
    }

    pub fn download(
        &self,
        url: String,
        folders: Vec<String>,
        file_name: String,
    ) -> Result<(), String> {
        let download_path = self
            .database
            .get_download_directory_sync()
            .map_err(|e| e.to_string())?;

        let mut download_dir = construct_download_path(download_path, folders);

        if !download_dir.exists() {
            fs::create_dir_all(&download_dir).map_err(|e| e.to_string())?;
        }

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|_| "Failed to download file".to_string())?;

        let headers = response.headers().clone();

        let ext = headers
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
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
            .or_else(|| {
                headers
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .and_then(mime_to_ext)
                    .map(|e| e.to_string())
            })
            .unwrap_or_else(|| ".bin".to_string());

        download_dir = download_dir.join(format!("{}{}", sanitize_file_name(&file_name), ext));

        let bytes: Bytes = response
            .bytes()
            .map_err(|_| "Failed to read response bytes".to_string())?;

        fs::write(&download_dir, &bytes).map_err(|_| "Failed to write file".to_string())?;

        Ok(())
    }
}

fn construct_download_path(base_path: String, folders: Vec<String>) -> PathBuf {
    let mut dir = Path::new(&base_path).to_path_buf();
    for folder in folders {
        dir = dir.join(sanitize_folder_uri(&folder));
    }
    dir
}

fn mime_to_ext(mime: &str) -> Option<&'static str> {
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
