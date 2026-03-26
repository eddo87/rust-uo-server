use log::{error, info};
use std::time::Duration;

pub mod error;
pub mod config;
mod huffman;
pub mod state;
mod tcp;
mod test_timers;
mod ticks;
pub mod timer;
pub mod shutdown;
pub mod connection;
pub mod connections;
pub mod movement;
pub mod speech;
pub mod world;
pub mod item;
pub mod inventory;
pub mod character;
pub mod persistence;
pub mod packets;
pub mod mobile;
pub mod combat;
pub mod encryption;
pub mod packet_validation;
pub mod skills;

// Phase 2: Game systems
pub mod account;
pub mod account_manager;
pub mod spells;
pub mod crafting;
pub mod status_packets;
pub mod container_packets;
pub mod targeting;
pub mod gump;
pub mod events;
pub mod commands;
pub mod map_files;
pub mod uop;
pub mod data_files;
pub mod loot;
pub mod vendor;
pub mod party;
pub mod guild;
pub mod housing;
pub mod weather;
pub mod resources;
pub mod effects;
pub mod quests;

fn main() {
    env_logger::init();
    info!("Starting rust-uo-server");

    let shutdown = shutdown::ShutdownSignal::new();

    let ctrlc_shutdown = shutdown.clone();
    ctrlc::set_handler(move || {
        info!("Shutdown signal received (Ctrl+C)");
        ctrlc_shutdown.request_shutdown();
    })
    .expect("Failed to set Ctrl+C handler");

    let timer_register_tx = timer::start();

    if let Err(e) = test_timers::start(timer_register_tx) {
        error!("Error starting test timers: {}", e);
    }

    let uo_data_dir = std::env::var("UO_DATA_DIR")
        .unwrap_or_else(|_| "C:/Program Files (x86)/Electronic Arts/Ultima Online Classic".to_string());
    let data_files = data_files::DataFiles::new(&uo_data_dir);
    match data_files.load_felucca() {
        Ok(map) => info!(
            "Felucca map loaded: {} MB of terrain data, {} bytes of statics index",
            map.map_len() / 1024 / 1024,
            map.map_len(),
        ),
        Err(e) => log::warn!("Could not load Felucca map data (set UO_DATA_DIR to fix): {}", e),
    }

    let connections = connections::ConnectionManager::new();

    if let Err(e) = tcp::start(connections) {
        error!("Error from TCP: {}", e);
    }

    info!("Server is running. Press Ctrl+C to shut down.");
    shutdown.wait_for_shutdown(Duration::from_millis(100));

    info!("Shutting down: allowing in-flight connections to finish...");
    std::thread::sleep(Duration::from_secs(2));
    info!("Server shut down gracefully.");
}
