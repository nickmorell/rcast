use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct AudioDownloader {
    cache_dir: PathBuf,
}

impl AudioDownloader {
    pub fn new() -> Self {
        let cache_dir = PathBuf::from("./cache/audio");
        std::fs::create_dir_all(&cache_dir).ok();

        Self { cache_dir }
    }

    fn hash_url(url: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        hasher.finish()
    }

    pub fn get_or_download(&self, url: &str, episode_id: i32) -> Result<PathBuf, String> {
        // Use a hash of the URL to ensure we download the correct audio
        let url_hash = Self::hash_url(url);
        let file_name = format!("episode_{}_{}.mp3", episode_id, url_hash);
        let file_path = self.cache_dir.join(&file_name);

        let old_file_name = format!("episode_{}.mp3", episode_id);
        let old_file_path = self.cache_dir.join(&old_file_name);

        // Check for URL metadata file to verify cached audio matches URL
        let metadata_file = self.cache_dir.join(format!("episode_{}.url", episode_id));

        if file_path.exists() {
            println!(
                "Audio already downloaded for episode {} (URL hash: {})",
                episode_id, url_hash
            );
            return Ok(file_path);
        }

        if old_file_path.exists() {
            println!(
                "Found old cache file for episode {}, checking if URL matches...",
                episode_id
            );
            let mut url_matches = false;

            if metadata_file.exists() {
                if let Ok(mut f) = File::open(&metadata_file) {
                    let mut cached_url = String::new();
                    if f.read_to_string(&mut cached_url).is_ok() && cached_url == url {
                        url_matches = true;
                        println!("URL matches, renaming old cache file to new format");
                        if std::fs::rename(&old_file_path, &file_path).is_ok() {
                            std::fs::remove_file(&metadata_file).ok();
                            return Ok(file_path);
                        }
                    }
                }
            }

            if !url_matches {
                println!("URL doesn't match or no metadata, deleting old cache file");
                std::fs::remove_file(&old_file_path).ok();
                std::fs::remove_file(&metadata_file).ok();
            }
        }

        println!("Downloading audio for episode {} from: {}", episode_id, url);
        let response =
            reqwest::blocking::get(url).map_err(|e| format!("Failed to download audio: {}", e))?;

        let bytes = response
            .bytes()
            .map_err(|e| format!("Failed to read audio bytes: {}", e))?;

        let mut file =
            File::create(&file_path).map_err(|e| format!("Failed to create audio file: {}", e))?;

        file.write_all(&bytes)
            .map_err(|e| format!("Failed to write audio file: {}", e))?;

        println!("Audio downloaded to: {:?}", file_path);
        Ok(file_path)
    }
}
