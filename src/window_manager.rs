//! Window tiling utility similar to Rectangle.
//! This module provides the interface for resizing and moving windows.
//!
//! TODO: Multi-monitor support — currently tiles windows on the default screen only.
//! Should use XRandR to detect the active output and compute per-monitor work areas.

#[allow(dead_code)]
pub enum WindowAction {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopLeftSixth,
    TopCenterSixth,
    TopRightSixth,
    BottomLeftSixth,
    BottomCenterSixth,
    BottomRightSixth,
    Maximize,
    Center,
}

#[cfg(target_os = "macos")]
pub fn perform_action(action: WindowAction) {
    let bounds_script = match action {
        WindowAction::LeftHalf => "{0, 23, desktop_width / 2, desktop_height}",
        WindowAction::RightHalf => "{desktop_width / 2, 23, desktop_width, desktop_height}",
        WindowAction::TopHalf => "{0, 23, desktop_width, desktop_height / 2}",
        WindowAction::BottomHalf => "{0, desktop_height / 2, desktop_width, desktop_height}",

        WindowAction::TopLeft => "{0, 23, desktop_width / 2, desktop_height / 2}",
        WindowAction::TopRight => "{desktop_width / 2, 23, desktop_width, desktop_height / 2}",
        WindowAction::BottomLeft => "{0, desktop_height / 2, desktop_width / 2, desktop_height}",
        WindowAction::BottomRight => {
            "{desktop_width / 2, desktop_height / 2, desktop_width, desktop_height}"
        }

        WindowAction::TopLeftSixth => "{0, 23, desktop_width / 3, desktop_height / 2}",
        WindowAction::TopCenterSixth => {
            "{desktop_width / 3, 23, desktop_width * 2 / 3, desktop_height / 2}"
        }
        WindowAction::TopRightSixth => {
            "{desktop_width * 2 / 3, 23, desktop_width, desktop_height / 2}"
        }
        WindowAction::BottomLeftSixth => {
            "{0, desktop_height / 2, desktop_width / 3, desktop_height}"
        }
        WindowAction::BottomCenterSixth => {
            "{desktop_width / 3, desktop_height / 2, desktop_width * 2 / 3, desktop_height}"
        }
        WindowAction::BottomRightSixth => {
            "{desktop_width * 2 / 3, desktop_height / 2, desktop_width, desktop_height}"
        }

        WindowAction::Maximize => "{0, 23, desktop_width, desktop_height}",
        WindowAction::Center => {
            "{desktop_width / 4, desktop_height / 4, desktop_width * 3 / 4, desktop_height * 3 / 4}"
        }
    };

    let script = format!(
        r#"
        tell application "Finder"
            set desktop_bounds to bounds of window of desktop
            set desktop_width to item 3 of desktop_bounds
            set desktop_height to item 4 of desktop_bounds
        end tell
        tell application "System Events"
            set frontApp to first application process whose frontmost is true
            set frontWindow to window 1 of frontApp
            set bounds of frontWindow to {}
        end tell
    "#,
        bounds_script
    );

    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn();
}

#[cfg(target_os = "linux")]
pub fn perform_action(action: WindowAction) {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{
        AtomEnum, ClientMessageEvent, ConfigureWindowAux, ConnectionExt, EventMask,
    };

    let (conn, screen_num) = match x11rb::connect(None) {
        Ok(c) => c,
        Err(_) => return,
    };
    let setup = conn.setup();
    let screen = &setup.roots[screen_num];

    let active_window_atom = match conn.intern_atom(false, b"_NET_ACTIVE_WINDOW") {
        Ok(cookie) => match cookie.reply() {
            Ok(reply) => reply.atom,
            Err(_) => return,
        },
        Err(_) => return,
    };

    let mut client_win = 0u32;
    if let Ok(cookie) = conn.get_property(
        false,
        screen.root,
        active_window_atom,
        AtomEnum::WINDOW,
        0,
        1,
    ) {
        if let Ok(reply) = cookie.reply() {
            client_win = reply.value32().and_then(|mut v| v.next()).unwrap_or(0);
        }
    }
    if client_win == 0 {
        if let Ok(cookie) = conn.get_input_focus() {
            if let Ok(reply) = cookie.reply() {
                client_win = reply.focus;
            }
        }
    }

    if client_win == 0 || client_win == 1 || client_win == screen.root {
        return;
    }

    if let Ok(cookie) = conn.get_property(
        false,
        client_win,
        AtomEnum::WM_CLASS,
        AtomEnum::STRING,
        0,
        1024,
    ) {
        if let Ok(reply) = cookie.reply() {
            let class = String::from_utf8_lossy(&reply.value);
            if class.contains("mun") {
                return;
            }
        }
    }

    let mut frame_win = client_win;
    while let Ok(cookie) = conn.query_tree(frame_win) {
        if let Ok(tree_reply) = cookie.reply() {
            if tree_reply.parent == screen.root || tree_reply.parent == 0 {
                break;
            }
            frame_win = tree_reply.parent;
        } else {
            break;
        }
    }

    let wm_state = match intern_atom(&conn, b"_NET_WM_STATE") {
        Some(a) => a,
        None => return,
    };
    let wm_max_vert = match intern_atom(&conn, b"_NET_WM_STATE_MAXIMIZED_VERT") {
        Some(a) => a,
        None => return,
    };
    let wm_max_horz = match intern_atom(&conn, b"_NET_WM_STATE_MAXIMIZED_HORZ") {
        Some(a) => a,
        None => return,
    };

    let mut is_maximized = false;
    if let Ok(cookie) = conn.get_property(false, client_win, wm_state, AtomEnum::ATOM, 0, 1024) {
        if let Ok(reply) = cookie.reply() {
            if let Some(mut atoms) = reply.value32() {
                is_maximized = atoms.any(|a| a == wm_max_vert || a == wm_max_horz);
            }
        }
    }

    if matches!(action, WindowAction::Maximize) {
        let action_code: u32 = if is_maximized { 0 } else { 1 };
        let data = [action_code, wm_max_vert, wm_max_horz, 0, 0];
        let event = ClientMessageEvent::new(32, client_win, wm_state, data);
        let _ = conn.send_event(
            false,
            screen.root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        );
        let _ = conn.flush();
        return;
    }

    if is_maximized {
        let data: [u32; 5] = [0, wm_max_vert, wm_max_horz, 0, 0];
        let event = ClientMessageEvent::new(32, client_win, wm_state, data);
        let _ = conn.send_event(
            false,
            screen.root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        );
        let _ = conn.flush();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let hints_atom = match intern_atom(&conn, b"WM_NORMAL_HINTS") {
        Some(a) => a,
        None => return,
    };
    let mut inc_w = 1i32;
    let mut inc_h = 1i32;
    let mut base_w = 0i32;
    let mut base_h = 0i32;
    let mut min_w = 0i32;
    let mut min_h = 0i32;

    if let Ok(cookie) = conn.get_property(false, client_win, hints_atom, AtomEnum::ANY, 0, 18) {
        if let Ok(reply) = cookie.reply() {
            if let Some(vals) = reply.value32() {
                let vals: Vec<u32> = vals.collect();
                if vals.len() >= 7 {
                    let flags = vals[0];
                    if flags & (1 << 4) != 0 {
                        min_w = vals[5] as i32;
                        min_h = vals[6] as i32;
                    }
                    if flags & (1 << 6) != 0 && vals.len() >= 11 {
                        inc_w = vals[9].max(1) as i32;
                        inc_h = vals[10].max(1) as i32;
                    }
                    if flags & (1 << 8) != 0 && vals.len() >= 17 {
                        base_w = vals[15] as i32;
                        base_h = vals[16] as i32;
                    } else {
                        base_w = min_w;
                        base_h = min_h;
                    }
                }
            }
        }
    }

    let (fl, fr, ft, fb) = (0, 0, 0, 0);

    let (sx, sy, sw, sh) = (
        0,
        0,
        screen.width_in_pixels as i32,
        screen.height_in_pixels as i32,
    );
    let usable_h = sh;

    let (nx, ny, mut nw, mut nh) = match action {
        WindowAction::LeftHalf => (sx, sy, sw / 2, usable_h),
        WindowAction::RightHalf => (sx + sw / 2, sy, sw / 2, usable_h),
        WindowAction::TopHalf => (sx, sy, sw, usable_h / 2),
        WindowAction::BottomHalf => (sx, sy + usable_h / 2, sw, usable_h / 2),
        WindowAction::TopLeft => (sx, sy, sw / 2, usable_h / 2),
        WindowAction::TopRight => (sx + sw / 2, sy, sw / 2, usable_h / 2),
        WindowAction::BottomLeft => (sx, sy + usable_h / 2, sw / 2, usable_h / 2),
        WindowAction::BottomRight => (sx + sw / 2, sy + usable_h / 2, sw / 2, usable_h / 2),
        WindowAction::TopLeftSixth => (sx, sy, sw / 3, usable_h / 2),
        WindowAction::TopCenterSixth => (sx + sw / 3, sy, sw / 3, usable_h / 2),
        WindowAction::TopRightSixth => (sx + sw * 2 / 3, sy, sw / 3, usable_h / 2),
        WindowAction::BottomLeftSixth => (sx, sy + usable_h / 2, sw / 3, usable_h / 2),
        WindowAction::BottomCenterSixth => (sx + sw / 3, sy + usable_h / 2, sw / 3, usable_h / 2),
        WindowAction::BottomRightSixth => {
            (sx + sw * 2 / 3, sy + usable_h / 2, sw / 3, usable_h / 2)
        }
        WindowAction::Center => (sx + sw / 4, sy + usable_h / 8, sw / 2, usable_h * 3 / 4),
        WindowAction::Maximize => unreachable!(),
    };

    let mut cw = nw - (fl + fr);
    let mut ch = nh - (ft + fb);

    if inc_w > 1 {
        cw = base_w + ((cw - base_w) / inc_w) * inc_w;
    }
    if inc_h > 1 {
        ch = base_h + ((ch - base_h) / inc_h) * inc_h;
    }

    cw = cw.max(min_w);
    ch = ch.max(min_h);

    nw = cw + fl + fr;
    nh = ch + ft + fb;

    let aux = ConfigureWindowAux::new()
        .x(nx)
        .y(ny)
        .width(nw as u32)
        .height(nh as u32);
    let _ = conn.configure_window(frame_win, &aux);

    if let Some(moveresize_atom) = intern_atom(&conn, b"_NET_MOVERESIZE_WINDOW") {
        let l0: u32 = 1 | (1 << 8) | (15 << 12);
        let data = [l0, nx as u32, ny as u32, cw as u32, ch as u32];
        let event = ClientMessageEvent::new(32, client_win, moveresize_atom, data);
        let _ = conn.send_event(
            false,
            screen.root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        );
    }

    let _ = conn.flush();
}

#[cfg(target_os = "linux")]
fn intern_atom(conn: &impl x11rb::protocol::xproto::ConnectionExt, name: &[u8]) -> Option<u32> {
    conn.intern_atom(false, name)
        .ok()?
        .reply()
        .ok()
        .map(|r| r.atom)
}
