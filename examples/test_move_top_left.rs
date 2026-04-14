use x11rb::connection::Connection;
use x11rb::protocol::xproto::{EventMask, ConfigureWindowAux, ClientMessageEvent, AtomEnum, ConnectionExt};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];

    // 1. Find Focused Window
    let focus = conn.get_input_focus()?.reply()?;
    let client_win = focus.focus;

    if client_win == screen.root || client_win == 0 {
        println!("No window focused.");
        return Ok(());
    }

    // 2. Find Frame Window (parent of client)
    let mut frame_win = client_win;
    let mut current = client_win;
    while current != screen.root && current != 0 {
        let tree = conn.query_tree(current)?.reply()?;
        if tree.parent == screen.root || tree.parent == 0 {
            frame_win = current;
            break;
        }
        current = tree.parent;
    }

    println!("Targeting Client Window: 0x{:x}", client_win);
    println!("Targeting Frame Window: 0x{:x}", frame_win);

    // 3. Get Frame Extents
    let extents_atom = conn.intern_atom(false, b"_NET_FRAME_EXTENTS")?.reply()?.atom;
    let mut fl = 0; let mut fr = 0; let mut ft = 0; let mut fb = 0;
    if let Ok(reply) = conn.get_property(false, client_win, extents_atom, AtomEnum::CARDINAL, 0, 4)?.reply() {
        if let Some(mut values) = reply.value32() {
            fl = values.next().unwrap_or(0) as i32;
            fr = values.next().unwrap_or(0) as i32;
            ft = values.next().unwrap_or(0) as i32;
            fb = values.next().unwrap_or(0) as i32;
        }
    }
    println!("Frame Extents: L={}, R={}, T={}, B={}", fl, fr, ft, fb);

    // 4. Get Work Area
    let workarea_atom = conn.intern_atom(false, b"_NET_WORKAREA")?.reply()?.atom;
    let mut sx = 0; let mut sy = 0;
    if let Ok(reply) = conn.get_property(false, screen.root, workarea_atom, AtomEnum::CARDINAL, 0, 4)?.reply() {
        if let Some(mut values) = reply.value32() {
            sx = values.next().unwrap_or(0) as i32;
            sy = values.next().unwrap_or(0) as i32;
        }
    }
    println!("Workarea Start: x={}, y={}", sx, sy);

    // 5. Current Geometry
    let geom = conn.get_geometry(frame_win)?.reply()?;
    println!("Current Frame Geometry: {}x{} at {},{}", geom.width, geom.height, geom.x, geom.y);

    // 6. Attempt Move to (0,0) - Absolute Top Left
    println!("\nMoving window to (0,0) via ConfigureWindow...");
    let aux = ConfigureWindowAux::new().x(0).y(0);
    conn.configure_window(frame_win, &aux)?;

    // 7. Backup: EWMH Move
    let moveresize_atom = conn.intern_atom(false, b"_NET_MOVERESIZE_WINDOW")?.reply()?.atom;
    let l0 = 1 | (10 << 8) | (3 << 12); // source=1, gravity=Static(10), mask=x|y (3)
    let data = [l0 as u32, 0, 0, 0, 0];
    let event = ClientMessageEvent::new(32, client_win, moveresize_atom, data);
    conn.send_event(false, screen.root, EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY, event)?;

    conn.flush()?;
    
    std::thread::sleep(std::time::Duration::from_millis(100));
    let new_geom = conn.get_geometry(frame_win)?.reply()?;
    println!("New Frame Geometry: {}x{} at {},{}", new_geom.width, new_geom.height, new_geom.x, new_geom.y);

    Ok(())
}
