mod adapters;
mod config;
mod domain;
mod launcher;
mod ports;

fn main() {
    env_logger::init();
    log::info!("Starting Mun - Launcher and Window Manager");
    if let Err(e) = launcher::run::<adapters::CurrentPlatform>() {
        log::error!("Failed to run launcher: {:?}", e);
    }
}
