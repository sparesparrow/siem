use axum::{
    Router,
    routing::{get, post},
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::config::Config;
use crate::security::SecurityManager;
use crate::scripts::ScriptsManager;
use crate::tickets::TicketsManager;
use crate::network::NetworkManager;
use crate::visualizations::VisualizationManager;

// Define application state that will be shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub security_manager: SecurityManager,
    pub scripts_manager: Arc<ScriptsManager>,
    pub tickets_manager: Arc<TicketsManager>,
    pub network_manager: Arc<NetworkManager>,
    pub visualization_manager: Arc<VisualizationManager>,
}

// Setup routes for API
pub fn setup_routes(
    config: Config,
    security_manager: SecurityManager,
    scripts_manager: ScriptsManager,
    tickets_manager: TicketsManager,
    network_manager: NetworkManager,
    visualization_manager: VisualizationManager,
) -> Router {
    let app_state = Arc::new(AppState {
        config,
        security_manager,
        scripts_manager: Arc::new(scripts_manager),
        tickets_manager: Arc::new(tickets_manager),
        network_manager: Arc::new(network_manager),
        visualization_manager: Arc::new(visualization_manager),
    });

    Router::new()
        .route("/", get(root_handler))
        .route("/api/health", get(health_check))

        // Network routes
        .route("/api/network/interfaces", get(get_interfaces))
        .route("/api/network/firewall/rules", get(get_firewall_rules))
        .route("/api/network/firewall/rules", post(add_firewall_rule))
        .route("/api/network/firewall/rules/:handle", delete(delete_firewall_rule))
        .route("/api/network/setup/:interface", post(setup_interface))

        // Visualization routes
        .route("/api/visualizations/network-graph", get(get_network_graph))
        .route("/api/visualizations/network-diagram/:format", get(get_network_diagram))
        .route("/api/visualizations/traffic-flows", get(get_traffic_flows))
        .route("/api/visualizations/traffic-stats", get(get_traffic_stats))
        .route("/api/visualizations/traffic-history/:interface", get(get_traffic_history))

        // Scripts routes
        .route("/api/scripts", get(list_scripts))
        .route("/api/scripts/:id", get(get_script))
        .route("/api/scripts", post(create_script))
        .route("/api/scripts/:id", put(update_script))
        .route("/api/scripts/:id", delete(delete_script))
        .route("/api/scripts/:id/execute", post(execute_script))

        // Tickets routes
        .route("/api/tickets", get(list_tickets))
        .route("/api/tickets/:id", get(get_ticket))
        .route("/api/tickets", post(create_ticket))
        .route("/api/tickets/:id", put(update_ticket))

        // Add the app state
        .with_state(app_state)
}

// Basic handlers
async fn root_handler() -> &'static str {
    "SIEM Admin Center API"
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// Network API handlers
async fn get_interfaces(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<crate::network::InterfaceInfo>>, StatusCode> {
    match state.network_manager.get_interfaces().await {
        Ok(interfaces) => Ok(Json(interfaces)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_firewall_rules(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<String>> {
    let rules = state.network_manager.get_nftables_rules().await;
    Json(rules)
}

#[derive(Deserialize)]
struct InterfaceConfig {
    dhcp: Option<bool>,
    address: Option<String>,
    nftables_zone: Option<String>,
}

async fn setup_interface(
    State(state): State<Arc<AppState>>,
    Path(interface_name): Path<String>,
    Json(config): Json<InterfaceConfig>,
) -> impl IntoResponse {
    let interface_config = crate::network::InterfaceConfig {
        name: interface_name,
        dhcp: config.dhcp,
        address: config.address,
        nftables_zone: config.nftables_zone,
    };

    match state.network_manager.setup_interface(&interface_config).await {
        Ok(_) => (StatusCode::OK, "Interface configured successfully"),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to configure interface: {}", e)),
    }
}

// Visualization API handlers
async fn get_network_graph(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let graph = state.visualization_manager.get_network_graph();
    (StatusCode::OK, Json(graph))
}

async fn get_traffic_flows(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let flows = state.visualization_manager.get_traffic_flows();
    (StatusCode::OK, Json(flows))
}

async fn get_traffic_stats(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let stats = state.visualization_manager.get_traffic_statistics();
    (StatusCode::OK, Json(stats))
}

async fn get_traffic_history(
    State(state): State<Arc<AppState>>,
    Path(interface): Path<String>,
) -> impl IntoResponse {
    let history = state.visualization_manager.get_traffic_history(&interface);
    (StatusCode::OK, Json(history))
}

async fn get_network_diagram(
    State(state): State<Arc<AppState>>,
    Path(format): Path<String>,
) -> impl IntoResponse {
    match state.visualization_manager.export_network_diagram(&format) {
        Ok(data) => {
            let content_type = match format.as_str() {
                "json" => "application/json",
                "dot" => "text/plain",
                _ => "application/octet-stream",
            };
            
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, content_type)],
                data
            )
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            e.into_bytes()
        ),
    }
}

#[derive(Deserialize)]
struct FirewallRuleRequest {
    chain: String,
    protocol: String,
    port: Option<u16>,
    source: Option<String>,
    action: String,
}

async fn add_firewall_rule(
    State(state): State<Arc<AppState>>,
    Json(rule): Json<FirewallRuleRequest>,
) -> impl IntoResponse {
    match state.network_manager.add_firewall_rule(
        &rule.chain,
        &rule.protocol,
        rule.port,
        rule.source.as_deref(),
        &rule.action
    ).await {
        Ok(_) => (StatusCode::CREATED, "Firewall rule added successfully"),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to add firewall rule: {}", e)),
    }
}

async fn delete_firewall_rule(
    State(state): State<Arc<AppState>>,
    Path(handle): Path<u32>,
) -> impl IntoResponse {
    match state.network_manager.delete_firewall_rule(handle).await {
        Ok(_) => (StatusCode::OK, "Firewall rule deleted successfully"),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete firewall rule: {}", e)),
    }
}

// Scripts API handlers - placeholder implementations
#[derive(Serialize, Deserialize)]
struct Script {
    id: String,
    name: String,
    content: String,
    description: Option<String>,
}

async fn list_scripts(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Placeholder implementation
    (StatusCode::OK, Json(Vec::<Script>::new()))
}

async fn get_script(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    // Placeholder implementation
    StatusCode::NOT_IMPLEMENTED
}

async fn create_script(
    State(_state): State<Arc<AppState>>,
    Json(_script): Json<Script>,
) -> impl IntoResponse {
    // Placeholder implementation
    StatusCode::NOT_IMPLEMENTED
}

async fn update_script(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<String>,
    Json(_script): Json<Script>,
) -> impl IntoResponse {
    // Placeholder implementation
    StatusCode::NOT_IMPLEMENTED
}

async fn delete_script(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    // Placeholder implementation
    StatusCode::NOT_IMPLEMENTED
}

async fn execute_script(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    // Placeholder implementation
    StatusCode::NOT_IMPLEMENTED
}

// Tickets API handlers - placeholder implementations
#[derive(Serialize, Deserialize)]
struct Ticket {
    id: String,
    title: String,
    description: String,
    status: String,
    priority: String,
    created_at: String,
    updated_at: String,
    assignee: Option<String>,
}

async fn list_tickets(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Placeholder implementation
    (StatusCode::OK, Json(Vec::<Ticket>::new()))
}

async fn get_ticket(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    // Placeholder implementation
    StatusCode::NOT_IMPLEMENTED
}

async fn create_ticket(
    State(_state): State<Arc<AppState>>,
    Json(_ticket): Json<Ticket>,
) -> impl IntoResponse {
    // Placeholder implementation
    StatusCode::NOT_IMPLEMENTED
}

async fn update_ticket(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<String>,
    Json(_ticket): Json<Ticket>,
) -> impl IntoResponse {
    // Placeholder implementation
    StatusCode::NOT_IMPLEMENTED
}