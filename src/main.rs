mod launcher;
mod window_manager;
mod app_scanner;
mod config;

fn main() {
    println!("Starting Mun - Launcher and Window Manager");
    // Start the launcher GUI
    // In a real application, you might want a background daemon listening to global hotkeys 
    // to toggle the launcher and trigger window manager actions.
    if let Err(e) = launcher::run() {
        eprintln!("Failed to run launcher: {:?}", e);
    }
}
