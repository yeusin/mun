use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, ConfigureWindowAux};

fn main() {
    if let Ok((conn, screen_num)) = x11rb::connect(None) {
        let setup = conn.setup();
        let screen = &setup.roots[screen_num];
        
        if let Ok(focus) = conn.get_input_focus() {
            if let Ok(focus_reply) = focus.reply() {
                let mut win = focus_reply.focus;
                println!("Focus win: {}", win);
                loop {
                    if win == 0 || win == 1 { break; }
                    if let Ok(tree) = conn.query_tree(win).and_then(|t| t.reply()) {
                        println!("Win: {}, Parent: {}", win, tree.parent);
                        if tree.parent == screen.root || tree.parent == 0 {
                            break;
                        }
                        win = tree.parent;
                    } else {
                        break;
                    }
                }
                println!("Top level win: {}", win);
            }
        }
    }
}
