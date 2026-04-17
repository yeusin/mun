use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConfigureWindowAux, ConnectionExt};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let focus_reply = conn.get_input_focus()?.reply()?;
    let focus_win = focus_reply.focus;
    if focus_win == root || focus_win == 0 {
        println!("No window focused.");
        return Ok(());
    }

    let mut frame_win = focus_win;
    let mut current = focus_win;
    while current != root && current != 0 {
        let tree = conn.query_tree(current)?.reply()?;
        if tree.parent == root || tree.parent == 0 {
            frame_win = current;
            break;
        }
        current = tree.parent;
    }

    let screen_w = screen.width_in_pixels as i32;
    let screen_h = screen.height_in_pixels as i32;
    let workarea_atom = conn.intern_atom(false, b"_NET_WORKAREA")?.reply()?.atom;
    let workarea_reply = conn
        .get_property(false, root, workarea_atom, AtomEnum::CARDINAL, 0, 4)?
        .reply()?;
    let (wa_x, wa_y, wa_w, wa_h) = match workarea_reply.value32() {
        Some(mut v) => (
            v.next().unwrap_or(0) as i32,
            v.next().unwrap_or(0) as i32,
            v.next().unwrap_or(screen_w as u32) as i32,
            v.next().unwrap_or(screen_h as u32) as i32,
        ),
        None => (0, 0, screen_w, screen_h),
    };

    let target_x = wa_x;
    let target_y = wa_y + wa_h / 2;
    let target_w = wa_w / 2;
    let target_h = wa_h / 2;
    println!(
        "target: x={} y={} w={} h={}",
        target_x, target_y, target_w, target_h
    );

    let mut x = target_x;
    let mut y = target_y;
    let mut w = target_w;
    let mut h = target_h;

    for i in 0..5 {
        let aux = ConfigureWindowAux::new()
            .x(x)
            .y(y)
            .width(w as u32)
            .height(h as u32);
        conn.configure_window(frame_win, &aux)?;
        conn.flush()?;
        std::thread::sleep(std::time::Duration::from_millis(200));

        if let Ok(cookie) = conn.get_geometry(frame_win) {
            if let Ok(g) = cookie.reply() {
                let dx = g.x as i32 - target_x;
                let dy = g.y as i32 - target_y;
                let dw = g.width as i32 - target_w;
                let dh = g.height as i32 - target_h;
                println!(
                    "[{}] actual: x={} y={} w={} h={}  delta: x={:+} y={:+} w={:+} h={:+}",
                    i, g.x, g.y, g.width, g.height, dx, dy, dw, dh
                );
                if dx == 0 && dy == 0 && dw == 0 && dh == 0 {
                    println!("converged at iteration {}", i);
                    return Ok(());
                }
                x = target_x - dx;
                y = target_y - dy;
                w = target_w - dw;
                h = target_h - dh;
            }
        }
    }

    println!("did not converge after 5 iterations");
    Ok(())
}
