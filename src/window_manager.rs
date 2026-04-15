/// Window tiling utility similar to Rectangle.
/// This module provides the interface for resizing and moving windows.

#[allow(dead_code)]
pub enum WindowAction {
    // Halves
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    // Quarters
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    // Sixths (2x3 grid)
    TopLeftSixth,
    TopCenterSixth,
    TopRightSixth,
    BottomLeftSixth,
    BottomCenterSixth,
    BottomRightSixth,
    // General
    Maximize,
    Center,
}

#[cfg(target_os = "macos")]
pub fn perform_action(action: WindowAction) {
    // macOS implementation using AppleScript
    let bounds_script = match action {
        WindowAction::LeftHalf => "{0, 23, desktop_width / 2, desktop_height}",
        WindowAction::RightHalf => "{desktop_width / 2, 23, desktop_width, desktop_height}",
        WindowAction::TopHalf => "{0, 23, desktop_width, desktop_height / 2}",
        WindowAction::BottomHalf => "{0, desktop_height / 2, desktop_width, desktop_height}",
        
        WindowAction::TopLeft => "{0, 23, desktop_width / 2, desktop_height / 2}",
        WindowAction::TopRight => "{desktop_width / 2, 23, desktop_width, desktop_height / 2}",
        WindowAction::BottomLeft => "{0, desktop_height / 2, desktop_width / 2, desktop_height}",
        WindowAction::BottomRight => "{desktop_width / 2, desktop_height / 2, desktop_width, desktop_height}",
        
        WindowAction::TopLeftSixth => "{0, 23, desktop_width / 3, desktop_height / 2}",
        WindowAction::TopCenterSixth => "{desktop_width / 3, 23, desktop_width * 2 / 3, desktop_height / 2}",
        WindowAction::TopRightSixth => "{desktop_width * 2 / 3, 23, desktop_width, desktop_height / 2}",
        WindowAction::BottomLeftSixth => "{0, desktop_height / 2, desktop_width / 3, desktop_height}",
        WindowAction::BottomCenterSixth => "{desktop_width / 3, desktop_height / 2, desktop_width * 2 / 3, desktop_height}",
        WindowAction::BottomRightSixth => "{desktop_width * 2 / 3, desktop_height / 2, desktop_width, desktop_height}",
        
        WindowAction::Maximize => "{0, 23, desktop_width, desktop_height}",
        WindowAction::Center => "{desktop_width / 4, desktop_height / 4, desktop_width * 3 / 4, desktop_height * 3 / 4}",
    };

    let script = format!(r#"
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
    "#, bounds_script);

    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn();
}

#[cfg(target_os = "linux")]
pub fn perform_action(action: WindowAction) {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt, ClientMessageEvent, EventMask, AtomEnum, ConfigureWindowAux};

    if let Ok((conn, screen_num)) = x11rb::connect(None) {
        let setup = conn.setup();
        let screen = &setup.roots[screen_num];
        
        // 1. Get Active Window
        let active_window_atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW").unwrap().reply().unwrap().atom;
        let mut client_win = 0;
        if let Ok(cookie) = conn.get_property(false, screen.root, active_window_atom, AtomEnum::WINDOW, 0, 1) {
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
            println!("No valid active window detected (ID: 0x{:x})", client_win);
            return;
        }

        // Detect if the active window is our own launcher (avoid tiling the launcher)
        if let Ok(cookie) = conn.get_property(false, client_win, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024) {
            if let Ok(reply) = cookie.reply() {
                let class = String::from_utf8_lossy(&reply.value);
                if class.contains("mun") {
                    println!("Active window is the launcher, ignoring tiling command.");
                    return; 
                }
            }
        }

        // 2. Find the frame window (direct child of root)
        let mut frame_win = client_win;
        loop {
            if let Ok(cookie) = conn.query_tree(frame_win) {
                if let Ok(tree_reply) = cookie.reply() {
                    if tree_reply.parent == screen.root || tree_reply.parent == 0 {
                        break;
                    }
                    frame_win = tree_reply.parent;
                } else { break; }
            } else { break; }
        }

        let wm_state = conn.intern_atom(false, b"_NET_WM_STATE").unwrap().reply().unwrap().atom;
        let wm_max_vert = conn.intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_VERT").unwrap().reply().unwrap().atom;
        let wm_max_horz = conn.intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_HORZ").unwrap().reply().unwrap().atom;

        // Check if maximized
        let mut is_maximized = false;
        if let Ok(cookie) = conn.get_property(false, client_win, wm_state, AtomEnum::ATOM, 0, 1024) {
            if let Ok(reply) = cookie.reply() {
                if let Some(mut atoms) = reply.value32() {
                    is_maximized = atoms.any(|a| a == wm_max_vert || a == wm_max_horz);
                }
            }
        }

        // 3. Handle Maximize Toggle
        if matches!(action, WindowAction::Maximize) {
            let action_code = if is_maximized { 0 } else { 1 };
            let data = [action_code, wm_max_vert, wm_max_horz, 0, 0];
            let event = ClientMessageEvent::new(32, client_win, wm_state, data);
            let _ = conn.send_event(false, screen.root, EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY, event);
            let _ = conn.flush();
            return;
        }

        // 4. For other actions, unmaximize IF needed and WAIT
        if is_maximized {
            println!("Window is maximized, sending unmaximize and waiting 100ms...");
            let data = [0, wm_max_vert, wm_max_horz, 0, 0];
            let event = ClientMessageEvent::new(32, client_win, wm_state, data);
            let _ = conn.send_event(false, screen.root, EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY, event);
            let _ = conn.flush();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // 5. Get Size Hints
        let hints_atom = conn.intern_atom(false, b"WM_NORMAL_HINTS").unwrap().reply().unwrap().atom;
        let mut inc_w = 1;
        let mut inc_h = 1;
        let mut base_w = 0;
        let mut base_h = 0;
        let mut min_w = 0;
        let mut min_h = 0;

        if let Ok(cookie) = conn.get_property(false, client_win, hints_atom, AtomEnum::ANY, 0, 18) {
            if let Ok(reply) = cookie.reply() {
                if let Some(vals) = reply.value32() {
                    let vals: Vec<u32> = vals.collect();
                    if vals.len() >= 7 {
                        let flags = vals[0];
                        let has_min = flags & (1 << 4) != 0;
                        let has_inc = flags & (1 << 6) != 0;
                        let has_base = flags & (1 << 8) != 0;

                        if has_min && vals.len() >= 7 {
                            min_w = vals[5] as i32;
                            min_h = vals[6] as i32;
                        }

                        if has_inc && vals.len() >= 11 {
                            inc_w = vals[9].max(1) as i32;
                            inc_h = vals[10].max(1) as i32;
                        }

                        if has_base && vals.len() >= 17 {
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

        // 5.1 Frame Extents ignored per user request (prevents issues in Xfce/xfwm4)
        let (fl, fr, ft, fb) = (0, 0, 0, 0);

        // 6. Work Area calculations
        // We default to the full screen size to allow absolute (0,0) tiling.
        // We purposefully ignore _NET_WORKAREA (which would include panel offsets)
        // so that the tiler is not "shunted" by the desktop environment.
        let (sx, sy, sw, sh) = (0, 0, screen.width_in_pixels as i32, screen.height_in_pixels as i32);

        let bottom_padding = 0; // Removed padding to allow exact screen-edge tiling
        let usable_h = sh - bottom_padding;

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
            WindowAction::BottomRightSixth => (sx + sw * 2 / 3, sy + usable_h / 2, sw / 3, usable_h / 2),
            WindowAction::Center => (sx + sw / 4, sy + usable_h / 8, sw / 2, usable_h * 3 / 4),
            WindowAction::Maximize => unreachable!(),
        };

        // 7. Adjust for Size Increments
        // Apply increments to the CLIENT window, then add frame extents back
        let mut cw = nw - (fl + fr);
        let mut ch = nh - (ft + fb);

        let (original_nw, original_nh) = (nw, nh);

        if inc_w > 1 { cw = base_w + ((cw - base_w) / inc_w) * inc_w; }
        if inc_h > 1 { ch = base_h + ((ch - base_h) / inc_h) * inc_h; }

        // Enforce min size
        cw = cw.max(min_w);
        ch = ch.max(min_h);

        nw = cw + fl + fr;
        nh = ch + ft + fb;

        let _dw = original_nw - nw;
        let _dh = original_nh - nh;

        let nx = nx;
        let ny = ny;

        println!("Executing tiling for 0x{:x}: x={}, y={}, w={}, h={} (client: {}x{})", client_win, nx, ny, nw, nh, cw, ch);

        // 8. Move/Resize
        // Try direct Frame Configuration first (most reliable for xfwm4/gnome)
        let aux = ConfigureWindowAux::new().x(nx).y(ny).width(nw as u32).height(nh as u32);
        let _ = conn.configure_window(frame_win, &aux);

        // Backup: EWMH message to Client Window
        let moveresize_atom = conn.intern_atom(false, b"_NET_MOVERESIZE_WINDOW").unwrap().reply().unwrap().atom;
        // source=1 (app), gravity=1 (NorthWest), mask=15 (x,y,w,h)
        // NorthWest gravity (1) ensures nx,ny are interpreted as frame coordinates, 
        // while cw,ch are the client window dimensions.
        let l0 = 1 | (1 << 8) | (15 << 12);
        let data = [l0 as u32, nx as u32, ny as u32, cw as u32, ch as u32];
        let event = ClientMessageEvent::new(32, client_win, moveresize_atom, data);
        let _ = conn.send_event(false, screen.root, EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY, event);

        let _ = conn.flush();
    }
}
