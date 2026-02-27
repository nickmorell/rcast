mod adapters;
mod application;
mod audio_downloader;
mod audio_player;
mod components;
mod database;
mod errors;
mod image_cache;
mod migrations;
mod pages;
mod ports;
mod rss_sync;
mod types;

use crate::adapters::rfd_folder_picker::RfdFolderPicker;
use crate::application::RCast;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn main() -> eframe::Result {
    let tokio_runtime =
        Arc::new(Runtime::new().expect("Failed to create Tokio runtime for async I/O"));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RCast - Podcast Player",
        options,
        Box::new(|cc| {
            // Load in Phosphor icons
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

            let folder_picker = Arc::new(RfdFolderPicker::new(tokio_runtime.clone()));
            Ok(Box::new(RCast::new(folder_picker)))
        }),
    )
}
