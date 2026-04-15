use crate::config::ConfigKey;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyManager,
};

pub fn register_config_hotkey(
    manager: &GlobalHotKeyManager,
    config_key: &ConfigKey,
) -> (u32, HotKey) {
    let mut modifiers = Modifiers::empty();
    for m in &config_key.modifiers {
        match m.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "meta" | "super" | "command" | "win" => modifiers |= Modifiers::META,
            _ => {}
        }
    }

    let code = str_to_code(&config_key.key);
    let hotkey = HotKey::new(Some(modifiers), code);
    if let Err(e) = manager.register(hotkey) {
        log::error!("Failed to register hotkey {:?}: {:?}", hotkey, e);
    }
    (hotkey.id(), hotkey)
}

pub fn apply_config_internal(state: &mut super::SharedState) {
    for hk in state.hotkeys.drain(..) {
        let _ = state.manager.unregister(hk);
    }
    state.tiling_ids.clear();

    let (l_id, l_hk) = register_config_hotkey(&state.manager, &state.config.launcher_hotkey);
    state.launcher_id = l_id;
    state.hotkeys.push(l_hk);

    for (action, key) in &state.config.window_actions {
        let (id, hk) = register_config_hotkey(&state.manager, key);
        state.tiling_ids.insert(id, action.clone());
        state.hotkeys.push(hk);
    }
}

fn str_to_code(s: &str) -> Code {
    match s.to_lowercase().as_str() {
        "space" => Code::Space,
        "arrowleft" | "left" => Code::ArrowLeft,
        "arrowright" | "right" => Code::ArrowRight,
        "arrowup" | "up" => Code::ArrowUp,
        "arrowdown" | "down" => Code::ArrowDown,
        "keya" | "a" => Code::KeyA,
        "keyb" | "b" => Code::KeyB,
        "keyc" | "c" => Code::KeyC,
        "keyd" | "d" => Code::KeyD,
        "keye" | "e" => Code::KeyE,
        "keyf" | "f" => Code::KeyF,
        "keyg" | "g" => Code::KeyG,
        "keyh" | "h" => Code::KeyH,
        "keyi" | "i" => Code::KeyI,
        "keyj" | "j" => Code::KeyJ,
        "keyk" | "k" => Code::KeyK,
        "keyl" | "l" => Code::KeyL,
        "keym" | "m" => Code::KeyM,
        "keyn" | "n" => Code::KeyN,
        "keyo" | "o" => Code::KeyO,
        "keyp" | "p" => Code::KeyP,
        "keyq" | "q" => Code::KeyQ,
        "keyr" | "r" => Code::KeyR,
        "keys" | "s" => Code::KeyS,
        "keyt" | "t" => Code::KeyT,
        "keyu" | "u" => Code::KeyU,
        "keyv" | "v" => Code::KeyV,
        "keyw" | "w" => Code::KeyW,
        "keyx" | "x" => Code::KeyX,
        "keyy" | "y" => Code::KeyY,
        "keyz" | "z" => Code::KeyZ,
        "num1" | "d1" | "1" => Code::Digit1,
        "num2" | "d2" | "2" => Code::Digit2,
        "num3" | "d3" | "3" => Code::Digit3,
        "num4" | "d4" | "4" => Code::Digit4,
        "num5" | "d5" | "5" => Code::Digit5,
        "num6" | "d6" | "6" => Code::Digit6,
        "num7" | "d7" | "7" => Code::Digit7,
        "num8" | "d8" | "8" => Code::Digit8,
        "num9" | "d9" | "9" => Code::Digit9,
        "num0" | "d0" | "0" => Code::Digit0,
        "enter" => Code::Enter,
        "escape" => Code::Escape,
        "tab" => Code::Tab,
        "backspace" => Code::Backspace,
        "insert" => Code::Insert,
        "delete" => Code::Delete,
        "home" => Code::Home,
        "end" => Code::End,
        "pageup" => Code::PageUp,
        "pagedown" => Code::PageDown,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        _ => Code::KeyA,
    }
}
