use anyhow::Result;
use axum::Router;
use clap::Parser;
use std::net::SocketAddr;
use std::fs;
use tokio;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod config;
mod printers;
mod scripts;
mod tickets;
mod api;
mod models;
mod security;
mod database;
mod network;
mod visualizations; // Added network and visualization modules

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Admin Center...");

    // Parse command line arguments
    let args = Args::parse();

    // Load configuration
    let config_path = &args.config;
    let config = if fs::metadata(config_path).is_ok() {
        info!("Loading configuration from {}", config_path);
        config::load(config_path)?
    } else {
        info!("Configuration file not found, creating default configuration");
        let default_config = config::default_config();
        // Make sure config directory exists
        fs::create_dir_all("config").ok();
        let config_str = toml::to_string_pretty(&default_config)?;
        fs::write(config_path, config_str)?;
        default_config
    };

    info!("Initializing security manager...");
    let security_manager = security::SecurityManager::new([0u8; 32]); // Production should use a proper key

    info!("Initializing database manager...");
    // Initialize database manager if a database URL is provided
    // This is temporarily commented out as database_url is not in the Config struct
    let db_manager = None;
    info!("Skipping database initialization for now");

    info!("Initializing network manager...");
    let network_manager = network::NetworkManager::new().await?;
    
    // For example purposes, create some default interface config
    let default_interfaces = vec![
        network::InterfaceConfig {
            name: "eth0".to_string(),
            dhcp: Some(true),
            address: None,
            nftables_zone: Some("wan".to_string()),
        },
        network::InterfaceConfig {
            name: "eth1".to_string(),
            dhcp: None,
            address: Some("192.168.1.1/24".to_string()),
            nftables_zone: Some("lan".to_string()),
        },
    ];
    
    network_manager.load_config(default_interfaces).await?;
    network_manager.initialize_nftables().await?;
    
    info!("Initializing visualization manager...");
    let visualization_manager = visualizations::VisualizationManager::new();
    
    // Start traffic monitoring in the background
    if let Err(e) = visualization_manager.start_traffic_monitoring() {
        warn!("Failed to start traffic monitoring: {}", e);
    } else {
        info!("Traffic monitoring started successfully");
    }

    info!("Initializing scripts manager...");
    let scripts_manager = scripts::ScriptsManager::new(&config.scripts_dir)?;

    info!("Initializing tickets manager...");
    let tickets_manager = tickets::TicketsManager::new();

    info!("Setting up API routes...");
    let app = api::setup_routes(
        config.clone(),
        security_manager,
        scripts_manager,
        tickets_manager,
        network_manager,
        visualization_manager
    );

    // Run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    info!("Listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}