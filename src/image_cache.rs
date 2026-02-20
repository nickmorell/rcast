use egui::{ColorImage, TextureHandle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct ImageCache {
    cache_dir: PathBuf,
    textures: Arc<Mutex<HashMap<String, TextureHandle>>>,
}

impl ImageCache {
    pub fn new() -> Self {
        let cache_dir = PathBuf::from("./cache/images");
        std::fs::create_dir_all(&cache_dir).ok();

        Self {
            cache_dir,
            textures: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_or_load(&self, url: &str, ctx: &egui::Context) -> Option<TextureHandle> {
        let cache_key = self.url_to_cache_key(url);

        {
            let textures = self.textures.lock().unwrap();
            if let Some(texture) = textures.get(&cache_key) {
                return Some(texture.clone());
            }
        }

        let cached_path = self.cache_dir.join(&cache_key);
        if cached_path.exists() {
            if let Some(texture) = self.load_image_from_path(&cached_path, ctx, &cache_key) {
                return Some(texture);
            }
        }

        self.download_and_cache(url, ctx, &cache_key)
    }

    fn url_to_cache_key(&self, url: &str) -> String {
        let hash = format!("{:x}", md5::compute(url.as_bytes()));
        let extension = url.rsplit('.').next().unwrap_or("jpg");
        format!("{}.{}", hash, extension)
    }

    fn download_and_cache(
        &self,
        url: &str,
        ctx: &egui::Context,
        cache_key: &str,
    ) -> Option<TextureHandle> {
        let response = reqwest::blocking::get(url).ok()?;
        let bytes = response.bytes().ok()?;

        let cached_path = self.cache_dir.join(cache_key);
        std::fs::write(&cached_path, &bytes).ok()?;

        self.load_image_from_path(&cached_path, ctx, cache_key)
    }

    fn load_image_from_path(
        &self,
        path: &Path,
        ctx: &egui::Context,
        cache_key: &str,
    ) -> Option<TextureHandle> {
        let img = image::open(path).ok()?;
        let size = [img.width() as usize, img.height() as usize];
        let image_buffer = img.to_rgba8();
        let pixels = image_buffer.as_flat_samples();

        let color_image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

        let texture = ctx.load_texture(cache_key, color_image, Default::default());

        {
            let mut textures = self.textures.lock().unwrap();
            textures.insert(cache_key.to_string(), texture.clone());
        }

        Some(texture)
    }

    pub fn get_default_texture(&self, ctx: &egui::Context) -> TextureHandle {
        let cache_key = "default_podcast_image";

        {
            let textures = self.textures.lock().unwrap();
            if let Some(texture) = textures.get(cache_key) {
                return texture.clone();
            }
        }

        let size = [200, 200];
        let mut pixels = vec![egui::Color32::from_rgb(60, 60, 65); size[0] * size[1]];

        for y in 0..size[1] {
            for x in 0..size[0] {
                let idx = y * size[0] + x;
                let intensity = ((x + y) % 40) as u8 * 3;
                pixels[idx] =
                    egui::Color32::from_rgb(60 + intensity, 60 + intensity, 65 + intensity);
            }
        }

        let color_image = ColorImage {
            size,
            pixels,
            source_size: egui::vec2(size[0] as f32, size[1] as f32),
        };

        let texture = ctx.load_texture(cache_key, color_image, Default::default());

        {
            let mut textures = self.textures.lock().unwrap();
            textures.insert(cache_key.to_string(), texture.clone());
        }

        texture
    }
}
