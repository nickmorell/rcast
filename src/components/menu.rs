use egui::Context;
use tokio::sync::mpsc::UnboundedSender;

use crate::commands::AppCommand;
use crate::state::AppState;
use crate::types::Page;

// Returns `true` if the user clicked "Add Podcast" this frame.
pub fn render(ctx: &Context, cmd_tx: &UnboundedSender<AppCommand>) -> bool {
    let mut open_add_podcast = false;

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Add Podcast").clicked() {
                    open_add_podcast = true;
                }

                if ui.button("Settings").clicked() {
                    let _ = cmd_tx.send(AppCommand::NavigateTo(Page::Settings));
                    ui.close();
                }

                ui.separator();

                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("About", |ui| {
                ui.label("RCast - Podcast Player");
                ui.label("Version 0.1.1");
            });
        });
    });

    open_add_podcast
}
