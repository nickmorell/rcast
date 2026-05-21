#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod adapters;
mod application;
mod audio_cache;
mod audio_player;
mod chapters;
mod design;
mod commands;
mod components;
mod db;
mod download_manager;
mod errors;
mod events;
mod hotkeys;
mod image_cache;
mod migrations;
mod notifier;
mod orchestrator;
mod pages;
mod ports;
mod state;
mod tray;
mod trim_silence;
mod types;
mod utils;

use adapters::rfd_file_picker::RfdFilePicker;
use adapters::rfd_folder_picker::RfdFolderPicker;
use application::RCast;
use audio_player::AudioPlayer;
use db::Database;
use download_manager::DownloadManager;
use hotkeys::HotkeyManager;
use orchestrator::Orchestrator;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tray::AppTray;

fn load_fonts() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    fonts.font_data.insert(
        "Inter-Regular".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "Inter-Medium".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Medium.ttf")).into(),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "Inter-Regular".to_owned());
    fonts.families.insert(
        egui::FontFamily::Name("Medium".into()),
        vec!["Inter-Medium".to_owned()],
    );
    fonts
}

fn main() -> eframe::Result {
    let tokio_runtime = Arc::new(Runtime::new().expect("Failed to create Tokio runtime"));

    // Load and prepare application icon
    let icon_data = include_bytes!("../assets/icons/icon-256x256.png");
    let icon_image = image::load_from_memory(icon_data)
        .expect("Failed to load icon")
        .to_rgba8();
    let icon = egui::IconData {
        rgba: icon_image.to_vec(),
        width: 256,
        height: 256,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_icon(icon),
        ..Default::default()
    };

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    let db = Database::new("rcast").expect("Failed to open database");
    let audio_player = AudioPlayer::new();
    let download_manager = DownloadManager::new(db.clone());

    tokio_runtime.spawn(
        Orchestrator::new(cmd_rx, event_tx, db, audio_player.clone(), download_manager).run(),
    );

    let folder_picker: Arc<RfdFolderPicker> = Arc::new(RfdFolderPicker::new(tokio_runtime.clone()));
    let file_picker = Arc::new(RfdFilePicker::new(tokio_runtime.clone()));

    eframe::run_native(
        "RCast - Podcast Player",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_fonts(load_fonts());

            cc.egui_ctx
                .set_visuals(design::visuals::build_visuals(&design::ThemeTokens::dark()));

            // System tray — failure is non-fatal (may not be supported on all platforms/configs).
            let tray = AppTray::new().ok();

            // Global hotkeys — failure is non-fatal.
            let hotkeys = HotkeyManager::new().ok();

            Ok(Box::new(RCast::new(
                cmd_tx,
                event_rx,
                audio_player,
                folder_picker,
                file_picker,
                tray,
                hotkeys,
            )))
        }),
    )
}
