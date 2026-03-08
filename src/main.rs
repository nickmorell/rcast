#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod adapters;
mod application;
mod audio_cache;
mod audio_player;
mod commands;
mod components;
mod db;
mod download_manager;
mod errors;
mod events;
mod image_cache;
mod migrations;
mod orchestrator;
mod pages;
mod ports;
mod state;
mod types;
mod utils;

use adapters::rfd_file_picker::RfdFilePicker;
use adapters::rfd_folder_picker::RfdFolderPicker;
use application::RCast;
use audio_player::AudioPlayer;
use db::Database;
use download_manager::DownloadManager;
use orchestrator::Orchestrator;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

fn main() -> eframe::Result {
    let tokio_runtime = Arc::new(Runtime::new().expect("Failed to create Tokio runtime"));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
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

    let folder_picker = Arc::new(RfdFolderPicker::new(tokio_runtime.clone()));
    let file_picker = Arc::new(RfdFilePicker::new(tokio_runtime.clone()));

    eframe::run_native(
        "RCast - Podcast Player",
        options,
        Box::new(move |cc| {
            // Register Phosphor icons.
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(RCast::new(
                cmd_tx,
                event_rx,
                audio_player,
                folder_picker,
                file_picker,
            )))
        }),
    )
}
