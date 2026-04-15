mod launcher;
mod window_manager;
mod app_scanner;
mod config;

fn main() {
    env_logger::init();
    log::info!("Starting Mun - Launcher and Window Manager");
    if let Err(e) = launcher::run() {
        log::error!("Failed to run launcher: {:?}", e);
    }
}
