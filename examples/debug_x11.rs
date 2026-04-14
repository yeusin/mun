use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, AtomEnum};

fn main() {
    println!("X11 Window Debug Tool");
    println!("---------------------");

    let (conn, screen_num) = x11rb::connect(None).expect("Failed to connect to X11");
    let setup = conn.setup();
    let screen = &setup.roots[screen_num];
    let root = screen.root;

    // 1. Get Active Window
    let active_atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW").unwrap().reply().unwrap().atom;
    let mut client_win = 0;
    if let Ok(cookie) = conn.get_property(false, root, active_atom, AtomEnum::WINDOW, 0, 1) {
        if let Ok(reply) = cookie.reply() {
            client_win = reply.value32().and_then(|mut v| v.next()).unwrap_or(0);
            println!("_NET_ACTIVE_WINDOW: 0x{:x}", client_win);
        }
    }

    if client_win == 0 {
        if let Ok(reply) = conn.get_input_focus().unwrap().reply() {
            client_win = reply.focus;
            println!("Fallback GetInputFocus: 0x{:x}", client_win);
        }
    }

    if client_win == 0 || client_win == root {
        println!("No active window found.");
        return;
    }

    // 2. Get Window Class and Name
    if let Ok(cookie) = conn.get_property(false, client_win, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024) {
        if let Ok(reply) = cookie.reply() {
            println!("WM_CLASS: {}", String::from_utf8_lossy(&reply.value));
        }
    }
    if let Ok(cookie) = conn.get_property(false, client_win, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024) {
        if let Ok(reply) = cookie.reply() {
            println!("WM_NAME: {}", String::from_utf8_lossy(&reply.value));
        }
    }

    // 3. Find Frame Window
    let mut frame_win = client_win;
    let mut path = vec![client_win];
    loop {
        let tree = conn.query_tree(frame_win).unwrap().reply().unwrap();
        if tree.parent == root || tree.parent == 0 {
            break;
        }
        frame_win = tree.parent;
        path.push(frame_win);
    }
    println!("Window Hierarchy (Client -> Root child): {:?}", path.iter().map(|w| format!("0x{:x}", w)).collect::<Vec<_>>());
    println!("Targeting Frame Window: 0x{:x}", frame_win);

    // 4. Check Size Hints (WM_NORMAL_HINTS)
    let hints_atom = conn.intern_atom(false, b"WM_NORMAL_HINTS").unwrap().reply().unwrap().atom;
    if let Ok(cookie) = conn.get_property(false, client_win, hints_atom, AtomEnum::ANY, 0, 100) {
        if let Ok(reply) = cookie.reply() {
            println!("WM_NORMAL_HINTS size: {} bytes", reply.value.len());
            if reply.value.len() >= 44 { 
                if let Some(vals) = reply.value32() {
                    let vals: Vec<u32> = vals.collect();
                    let flags = vals[0];
                    println!("  Flags: 0x{:x}", flags);
                    if flags & (1 << 4) != 0 && vals.len() > 6 { // PMinSize
                        println!("  Min Size: {}x{}", vals[5], vals[6]);
                    }
                    if flags & (1 << 6) != 0 && vals.len() > 10 { // PResizeInc
                        println!("  Width Increment: {}", vals[9]);
                        println!("  Height Increment: {}", vals[10]);
                    }
                    if flags & (1 << 7) != 0 && vals.len() > 14 { // PAspect
                        println!("  Min Aspect: {}/{}", vals[11], vals[12]);
                        println!("  Max Aspect: {}/{}", vals[13], vals[14]);
                    }
                    if flags & (1 << 8) != 0 && vals.len() > 16 { // PBaseSize
                        println!("  Base Width: {}", vals[15]);
                        println!("  Base Height: {}", vals[16]);
                    }
                }
            }
        }
    }

    // 5. Check Frame Extents
    let extents_atom = conn.intern_atom(false, b"_NET_FRAME_EXTENTS").unwrap().reply().unwrap().atom;
    if let Ok(cookie) = conn.get_property(false, client_win, extents_atom, AtomEnum::CARDINAL, 0, 4) {
        if let Ok(reply) = cookie.reply() {
            if let Some(extents) = reply.value32() {
                let extents: Vec<u32> = extents.collect();
                if extents.len() == 4 {
                    println!("_NET_FRAME_EXTENTS: L={}, R={}, T={}, B={}", extents[0], extents[1], extents[2], extents[3]);
                }
            }
        }
    }

    // 6. Get Current Geometries
    let c_geom = conn.get_geometry(client_win).unwrap().reply().unwrap();
    let f_geom = conn.get_geometry(frame_win).unwrap().reply().unwrap();
    println!("Client Geometry: {}x{} at {},{}", c_geom.width, c_geom.height, c_geom.x, c_geom.y);
    println!("Frame Geometry: {}x{} at {},{}", f_geom.width, f_geom.height, f_geom.x, f_geom.y);

    println!("\nRun this script with your target window focused.");
}
