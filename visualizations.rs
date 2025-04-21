
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use geo::{Point, LineString, MultiLineString, Polygon};
use uuid::Uuid;
use crate::network::InterfaceInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkNode {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    pub position: Point<f64>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Router,
    Switch,
    Firewall,
    Server,
    Client,
    Internet,
    VirtualMachine,
    Container,
    Wireless,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLink {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub link_type: LinkType,
    pub path: LineString<f64>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkType {
    Ethernet,
    Fiber,
    Wireless,
    VPN,
    VLAN,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkZone {
    pub id: String,
    pub name: String,
    pub zone_type: ZoneType,
    pub boundary: Polygon<f64>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZoneType {
    Public,
    Private,
    DMZ,
    Secure,
    Restricted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkGraph {
    pub nodes: Vec<NetworkNode>,
    pub links: Vec<NetworkLink>,
    pub zones: Vec<NetworkZone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficFlow {
    pub source: String,
    pub destination: String,
    pub protocol: String,
    pub port: u16,
    pub bytes: u64,
    pub packets: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct VisualizationManager {
    network_graph: Arc<Mutex<NetworkGraph>>,
    traffic_flows: Arc<Mutex<Vec<TrafficFlow>>>,
    traffic_stats: Arc<Mutex<HashMap<String, InterfaceTrafficStats>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceTrafficStats {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub history: Vec<TrafficDataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficDataPoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

impl VisualizationManager {
    pub fn new() -> Self {
        // Create an empty network graph
        let network_graph = NetworkGraph {
            nodes: Vec::new(),
            links: Vec::new(),
            zones: Vec::new(),
        };
        
        Self {
            network_graph: Arc::new(Mutex::new(network_graph)),
            traffic_flows: Arc::new(Mutex::new(Vec::new())),
            traffic_stats: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn start_traffic_monitoring(&self) -> Result<(), std::io::Error> {
        let traffic_stats = self.traffic_stats.clone();
        
        // Start a background task to collect traffic statistics
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                if let Err(e) = Self::collect_traffic_stats(traffic_stats.clone()).await {
                    eprintln!("Error collecting traffic stats: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    async fn collect_traffic_stats(traffic_stats: Arc<Mutex<HashMap<String, InterfaceTrafficStats>>>) -> Result<(), std::io::Error> {
        // On Linux, read from /proc/net/dev
        let content = tokio::fs::read_to_string("/proc/net/dev").await?;
        
        let mut stats = traffic_stats.lock().unwrap();
        let now = chrono::Utc::now();
        
        for line in content.lines().skip(2) { // Skip the header lines
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 17 {
                continue;
            }
            
            let name = parts[0].trim_end_matches(':').to_string();
            
            // Parse traffic statistics
            let rx_bytes = parts[1].parse::<u64>().unwrap_or(0);
            let rx_packets = parts[2].parse::<u64>().unwrap_or(0);
            let tx_bytes = parts[9].parse::<u64>().unwrap_or(0);
            let tx_packets = parts[10].parse::<u64>().unwrap_or(0);
            
            // Update or create stats for this interface
            let entry = stats.entry(name.clone()).or_insert_with(|| InterfaceTrafficStats {
                name: name.clone(),
                rx_bytes: 0,
                tx_bytes: 0,
                rx_packets: 0,
                tx_packets: 0,
                timestamp: now,
                history: Vec::new(),
            });
            
            // Save historical data point (keep last 1000 points)
            entry.history.push(TrafficDataPoint {
                timestamp: entry.timestamp,
                rx_bytes: entry.rx_bytes,
                tx_bytes: entry.tx_bytes,
            });
            
            if entry.history.len() > 1000 {
                entry.history.remove(0);
            }
            
            // Update current values
            entry.rx_bytes = rx_bytes;
            entry.tx_bytes = tx_bytes;
            entry.rx_packets = rx_packets;
            entry.tx_packets = tx_packets;
            entry.timestamp = now;
        }
        
        Ok(())
    }
    
    pub fn get_network_graph(&self) -> NetworkGraph {
        self.network_graph.lock().unwrap().clone()
    }
    
    pub fn update_from_interfaces(&self, interfaces: &[InterfaceInfo]) {
        let mut graph = self.network_graph.lock().unwrap();
        
        // Create a central router node if it doesn't exist
        let router_id = "router-main".to_string();
        if !graph.nodes.iter().any(|n| n.id == router_id) {
            graph.nodes.push(NetworkNode {
                id: router_id.clone(),
                name: "Main Router".to_string(),
                node_type: NodeType::Router,
                position: Point::new(0.0, 0.0),
                properties: HashMap::new(),
            });
        }
        
        // Create nodes for each interface
        for (i, interface) in interfaces.iter().enumerate() {
            let interface_id = format!("interface-{}", interface.name);
            
            // Check if the node already exists
            if !graph.nodes.iter().any(|n| n.id == interface_id) {
                // Calculate position in a circle around the router
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / (interfaces.len() as f64);
                let distance = 100.0;
                let x = distance * angle.cos();
                let y = distance * angle.sin();
                
                // Determine node type based on interface name
                let node_type = if interface.name.starts_with("eth") {
                    NodeType::Switch
                } else if interface.name.starts_with("wlan") {
                    NodeType::Wireless
                } else {
                    NodeType::Client
                };
                
                // Create a new node
                let mut properties = HashMap::new();
                properties.insert("mac_address".to_string(), interface.mac_address.clone());
                properties.insert("is_up".to_string(), interface.is_up.to_string());
                
                let node = NetworkNode {
                    id: interface_id.clone(),
                    name: interface.name.clone(),
                    node_type,
                    position: Point::new(x, y),
                    properties,
                };
                
                graph.nodes.push(node);
                
                // Create a link between the router and this interface
                let link = NetworkLink {
                    id: Uuid::new_v4().to_string(),
                    source_id: router_id.clone(),
                    target_id: interface_id.clone(),
                    link_type: LinkType::Ethernet,
                    path: LineString::from(vec![(0.0, 0.0), (x, y)]),
                    properties: HashMap::new(),
                };
                
                graph.links.push(link);
            }
            
            // Update properties for the interface node
            if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == interface_id) {
                node.properties.insert("is_up".to_string(), interface.is_up.to_string());
                
                // Add IP addresses
                for (i, addr) in interface.addresses.iter().enumerate() {
                    node.properties.insert(format!("ip_address_{}", i), addr.clone());
                }
            }
        }
    }
    
    pub fn add_traffic_flow(&self, flow: TrafficFlow) {
        let mut flows = self.traffic_flows.lock().unwrap();
        flows.push(flow);
        
        // Keep only the latest 1000 flows to avoid using too much memory
        if flows.len() > 1000 {
            flows.remove(0);
        }
    }
    
    pub fn get_traffic_flows(&self) -> Vec<TrafficFlow> {
        self.traffic_flows.lock().unwrap().clone()
    }
    
    pub fn create_zone(&self, name: &str, zone_type: ZoneType, nodes: &[String]) {
        let mut graph = self.network_graph.lock().unwrap();
        
        // Find nodes in this zone
        let zone_nodes: Vec<&NetworkNode> = graph.nodes.iter()
            .filter(|n| nodes.contains(&n.id))
            .collect();
        
        if zone_nodes.is_empty() {
            return;
        }
        
        // Calculate a simple convex hull approximation for the zone boundary
        // For simplicity, we'll just create a rectangle that encompasses all nodes
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        
        for node in &zone_nodes {
            let x = node.position.x();
            let y = node.position.y();
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        
        // Add some padding
        min_x -= 20.0;
        min_y -= 20.0;
        max_x += 20.0;
        max_y += 20.0;
        
        // Create polygon for the zone
        let exterior = LineString::from(vec![
            (min_x, min_y),
            (max_x, min_y),
            (max_x, max_y),
            (min_x, max_y),
            (min_x, min_y),
        ]);
        
        let polygon = Polygon::new(exterior, vec![]);
        
        // Create the zone
        let zone = NetworkZone {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            zone_type,
            boundary: polygon,
            properties: HashMap::new(),
        };
        
        graph.zones.push(zone);
    }
    
    pub fn generate_topology_json(&self) -> String {
        let graph = self.network_graph.lock().unwrap();
        serde_json::to_string_pretty(&*graph).unwrap_or_else(|_| "{}".to_string())
    }
    
    pub fn generate_traffic_flow_json(&self) -> String {
        let flows = self.traffic_flows.lock().unwrap();
        serde_json::to_string_pretty(&*flows).unwrap_or_else(|_| "[]".to_string())
    }
    
    pub fn get_traffic_statistics(&self) -> HashMap<String, InterfaceTrafficStats> {
        self.traffic_stats.lock().unwrap().clone()
    }
    
    pub fn get_traffic_history(&self, interface_name: &str) -> Vec<TrafficDataPoint> {
        let stats = self.traffic_stats.lock().unwrap();
        if let Some(interface) = stats.get(interface_name) {
            interface.history.clone()
        } else {
            Vec::new()
        }
    }
    
    pub fn export_network_diagram(&self, format: &str) -> Result<Vec<u8>, String> {
        let graph = self.network_graph.lock().unwrap();
        
        match format {
            "json" => {
                match serde_json::to_vec_pretty(&*graph) {
                    Ok(data) => Ok(data),
                    Err(e) => Err(format!("Failed to serialize graph: {}", e)),
                }
            },
            "dot" => {
                // Generate Graphviz DOT format
                let mut dot = String::new();
                dot.push_str("digraph network {\n");
                dot.push_str("  rankdir=TB;\n");
                dot.push_str("  node [shape=box, style=filled, fillcolor=lightblue];\n\n");
                
                // Add nodes
                for node in &graph.nodes {
                    let node_type = format!("{:?}", node.node_type).to_lowercase();
                    let label = format!("{} ({})", node.name, node_type);
                    
                    dot.push_str(&format!("  \"{}\" [label=\"{}\"];\n", node.id, label));
                }
                
                // Add edges
                for link in &graph.links {
                    let link_type = format!("{:?}", link.link_type).to_lowercase();
                    dot.push_str(&format!("  \"{}\" -> \"{}\" [label=\"{}\"];\n", 
                                          link.source_id, link.target_id, link_type));
                }
                
                dot.push_str("}\n");
                
                Ok(dot.into_bytes())
            },
            _ => Err(format!("Unsupported format: {}", format)),
        }
    }
}
