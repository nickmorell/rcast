mod application;
mod audio_downloader;
mod audio_player;
mod components;
mod database;
mod errors;
mod image_cache;
mod migrations;
mod pages;
mod rss_sync;
mod types;

use crate::application::RCast;

fn main() -> eframe::Result {
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

            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(RCast::new()))
        }),
    )
}
