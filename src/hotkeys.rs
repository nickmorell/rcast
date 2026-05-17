use crate::commands::AppCommand;
use crate::types::HotkeySettings;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone, Copy)]
enum HotkeyAction {
    TogglePlayback,
    Next,
    SkipForward,
    SkipBackward,
}

pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    registered: Vec<HotKey>,
    actions: HashMap<u32, HotkeyAction>,
}

impl HotkeyManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            manager: GlobalHotKeyManager::new()?,
            registered: Vec::new(),
            actions: HashMap::new(),
        })
    }

    pub fn apply_settings(&mut self, settings: &HotkeySettings) {
        for hk in self.registered.drain(..) {
            let _ = self.manager.unregister(hk);
        }
        self.actions.clear();

        self.try_register(&settings.play_pause, HotkeyAction::TogglePlayback);
        self.try_register(&settings.next, HotkeyAction::Next);
        self.try_register(&settings.skip_forward, HotkeyAction::SkipForward);
        self.try_register(&settings.skip_backward, HotkeyAction::SkipBackward);
    }

    fn try_register(&mut self, hotkey_str: &str, action: HotkeyAction) {
        if hotkey_str.is_empty() {
            return;
        }
        if let Some(hk) = parse_hotkey(hotkey_str) {
            let id = hk.id();
            // register() takes by value; clone so we can keep a copy for unregister later.
            if self.manager.register(hk.clone()).is_ok() {
                self.actions.insert(id, action);
                self.registered.push(hk);
            }
        }
    }

    pub fn poll(&self, cmd_tx: &UnboundedSender<AppCommand>, app_focused: bool) {
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if !app_focused || event.state == HotKeyState::Released {
                continue;
            }
            if let Some(action) = self.actions.get(&event.id) {
                let cmd = match action {
                    HotkeyAction::TogglePlayback => AppCommand::TogglePlayback,
                    HotkeyAction::Next => AppCommand::PlayNextInQueue,
                    HotkeyAction::SkipForward => AppCommand::JumpForward,
                    HotkeyAction::SkipBackward => AppCommand::JumpBackward,
                };
                let _ = cmd_tx.send(cmd);
            }
        }
    }
}

fn parse_hotkey(s: &str) -> Option<HotKey> {
    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in s.split('+').map(str::trim) {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "super" | "win" | "cmd" => modifiers |= Modifiers::SUPER,
            other => {
                key_code = str_to_code(other);
            }
        }
    }

    key_code.map(|code| {
        HotKey::new(
            if modifiers.is_empty() {
                None
            } else {
                Some(modifiers)
            },
            code,
        )
    })
}

fn str_to_code(s: &str) -> Option<Code> {
    use Code::*;
    Some(match s {
        "space" => Space,
        "enter" | "return" => Enter,
        "left" => ArrowLeft,
        "right" => ArrowRight,
        "up" => ArrowUp,
        "down" => ArrowDown,
        "a" => KeyA,
        "b" => KeyB,
        "c" => KeyC,
        "d" => KeyD,
        "e" => KeyE,
        "f" => KeyF,
        "g" => KeyG,
        "h" => KeyH,
        "i" => KeyI,
        "j" => KeyJ,
        "k" => KeyK,
        "l" => KeyL,
        "m" => KeyM,
        "n" => KeyN,
        "o" => KeyO,
        "p" => KeyP,
        "q" => KeyQ,
        "r" => KeyR,
        "s" => KeyS,
        "t" => KeyT,
        "u" => KeyU,
        "v" => KeyV,
        "w" => KeyW,
        "x" => KeyX,
        "y" => KeyY,
        "z" => KeyZ,
        "0" => Digit0,
        "1" => Digit1,
        "2" => Digit2,
        "3" => Digit3,
        "4" => Digit4,
        "5" => Digit5,
        "6" => Digit6,
        "7" => Digit7,
        "8" => Digit8,
        "9" => Digit9,
        "f1" => F1,
        "f2" => F2,
        "f3" => F3,
        "f4" => F4,
        "f5" => F5,
        "f6" => F6,
        "f7" => F7,
        "f8" => F8,
        "f9" => F9,
        "f10" => F10,
        "f11" => F11,
        "f12" => F12,
        _ => return None,
    })
}
