use crate::commands::AppCommand;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tokio::sync::mpsc::UnboundedSender;
use tray_icon::{TrayIcon, TrayIconBuilder};

pub struct AppTray {
    _icon: TrayIcon,
    play_pause_id: muda::MenuId,
    next_id: muda::MenuId,
    open_id: muda::MenuId,
    quit_id: muda::MenuId,
}

impl AppTray {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let play_pause = MenuItem::new("Play / Pause", true, None);
        let next = MenuItem::new("Next", true, None);
        let separator = PredefinedMenuItem::separator();
        let open = MenuItem::new("Open RCast", true, None);
        let quit = MenuItem::new("Quit", true, None);

        let play_pause_id = play_pause.id().clone();
        let next_id = next.id().clone();
        let open_id = open.id().clone();
        let quit_id = quit.id().clone();

        let menu = Menu::new();
        menu.append(&play_pause)?;
        menu.append(&next)?;
        menu.append(&separator)?;
        menu.append(&open)?;
        menu.append(&quit)?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("RCast")
            .with_icon(make_icon())
            .build()?;

        Ok(Self {
            _icon: tray,
            play_pause_id,
            next_id,
            open_id,
            quit_id,
        })
    }

    pub fn poll(&self, cmd_tx: &UnboundedSender<AppCommand>, ctx: &egui::Context) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.play_pause_id {
                let _ = cmd_tx.send(AppCommand::TogglePlayback);
            } else if event.id == self.next_id {
                let _ = cmd_tx.send(AppCommand::PlayNextInQueue);
            } else if event.id == self.open_id {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else if event.id == self.quit_id {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

fn make_icon() -> tray_icon::Icon {
    let icon_data = include_bytes!("../assets/icons/icon-128x128.png");
    let icon_image = image::load_from_memory(icon_data)
        .expect("Failed to load tray icon")
        .to_rgba8();
    let (width, height) = icon_image.dimensions();
    tray_icon::Icon::from_rgba(icon_image.to_vec(), width, height)
        .expect("valid icon data")
}
