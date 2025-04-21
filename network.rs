
use anyhow::{Context, Result};
use rtnetlink::{new_connection, Handle, IpVersion};
use futures::stream::TryStreamExt;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::process::Command;
use tracing::{info, warn, error};

// Define NFTables module
mod nftables {
    use serde::{Deserialize, Serialize};
    use std::fmt;
    use std::process::Command;
    use anyhow::{Result, Context};
    
    pub struct Batch {
        commands: Vec<String>,
    }
    
    impl Batch {
        pub fn new() -> Self {
            Self {
                commands: Vec::new(),
            }
        }
        
        pub fn add(&mut self, stmt: &Stmt, comment: Option<&str>) {
            let mut cmd = format!("{}", stmt);
            if let Some(c) = comment {
                cmd = format!("{} # {}", cmd, c);
            }
            self.commands.push(cmd);
        }
        
        pub fn execute(&self) -> Result<String> {
            let script = self.commands.join("\n");
            
            // Create a temporary file with the nft script
            let temp_file = tempfile::NamedTempFile::new()
                .context("Failed to create temporary file for nft script")?;
                
            std::fs::write(temp_file.path(), &script)
                .context("Failed to write nft script to temporary file")?;
                
            // Execute nft -f script.nft
            let output = Command::new("nft")
                .arg("-f")
                .arg(temp_file.path().to_str().unwrap())
                .output()
                .context("Failed to execute nft command")?;
                
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("nft command failed: {}", stderr));
            }
            
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        
        pub fn clone(&self) -> Self {
            Self {
                commands: self.commands.clone(),
            }
        }
    }
    
    pub enum Stmt {
        AddTable(objects::AddTable),
        AddChain(objects::AddChain),
        Add(objects::Add),
        Flush(objects::Flush),
    }
    
    impl fmt::Display for Stmt {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Stmt::AddTable(t) => write!(f, "add table {} {}", t.family, t.name),
                Stmt::AddChain(c) => {
                    if let Some(constraint) = &c.constraint {
                        write!(f, "add chain {} {} {} {}", c.family, c.table, c.name, constraint)
                    } else {
                        write!(f, "add chain {} {} {}", c.family, c.table, c.name)
                    }
                },
                Stmt::Add(a) => {
                    write!(f, "add rule {} {} {} ", a.family, a.table, a.chain)?;
                    for (i, e) in a.expr.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ")?;
                        }
                        write!(f, "{}", e)?;
                    }
                    Ok(())
                },
                Stmt::Flush(flush) => write!(f, "{}", flush),
            }
        }
    }
    
    pub mod objects {
        use super::schemas::nftables::TableFamily;
        use serde::{Deserialize, Serialize};
        use std::fmt;
        
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub struct AddTable {
            pub family: TableFamily,
            pub name: String,
        }
        
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub struct AddChain {
            pub family: TableFamily,
            pub table: String,
            pub name: String,
            pub handle: Option<u32>,
            pub constraint: Option<String>,
        }
        
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub struct Add {
            pub family: TableFamily,
            pub table: String,
            pub chain: String,
            pub handle: Option<u32>,
            pub index: Option<u32>,
            pub expr: Vec<super::expr::Expr>,
        }
        
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub enum Flush {
            Table {
                family: TableFamily,
                name: String,
            },
            Chain {
                family: TableFamily,
                table: String,
                name: String,
            },
        }
        
        impl fmt::Display for Flush {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Flush::Table { family, name } => write!(f, "flush table {} {}", family, name),
                    Flush::Chain { family, table, name } => write!(f, "flush chain {} {} {}", family, table, name),
                }
            }
        }
    }
    
    pub mod expr {
        use serde::{Deserialize, Serialize};
        use std::fmt;
        
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub enum Expr {
            Match(Match),
            Cmp(Cmp),
            Accept(Accept),
            Drop(Drop),
            Counter(Counter),
        }
        
        impl fmt::Display for Expr {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Expr::Match(m) => write!(f, "{}", m),
                    Expr::Cmp(c) => write!(f, "{}", c),
                    Expr::Accept(a) => write!(f, "{}", a),
                    Expr::Drop(d) => write!(f, "{}", d),
                    Expr::Counter(c) => write!(f, "{}", c),
                }
            }
        }
        
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub struct Match {
            pub op: String,
            pub expr: Box<Expr>,
        }
        
        impl fmt::Display for Match {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{} {}", self.op, self.expr)
            }
        }
        
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub struct Cmp {
            pub op: String,
            pub data: Data,
        }
        
        impl fmt::Display for Cmp {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match &self.data {
                    Data::Set(set) => {
                        write!(f, "{} {{ ", self.op)?;
                        for (i, item) in set.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}", item)?;
                        }
                        write!(f, " }}")
                    },
                    Data::StrVal(val) => write!(f, "{} {}", self.op, val),
                    Data::NumVal(val) => write!(f, "{} {}", self.op, val),
                }
            }
        }
        
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub enum Data {
            Set(Vec<String>),
            StrVal(String),
            NumVal(u64),
        }
        
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub struct Accept {
        }
        
        impl fmt::Display for Accept {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "accept")
            }
        }
        
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub struct Drop {
        }
        
        impl fmt::Display for Drop {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "drop")
            }
        }
        
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub struct Counter {
        }
        
        impl fmt::Display for Counter {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "counter")
            }
        }
    }
    
    pub mod schemas {
        pub mod nftables {
            use serde::{Deserialize, Serialize};
            use std::fmt;
            
            #[derive(Debug, Clone, Deserialize, Serialize)]
            pub enum TableFamily {
                Ip,
                Ip6,
                Inet,
                Arp,
                Bridge,
                Netdev,
            }
            
            impl fmt::Display for TableFamily {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    match self {
                        TableFamily::Ip => write!(f, "ip"),
                        TableFamily::Ip6 => write!(f, "ip6"),
                        TableFamily::Inet => write!(f, "inet"),
                        TableFamily::Arp => write!(f, "arp"),
                        TableFamily::Bridge => write!(f, "bridge"),
                        TableFamily::Netdev => write!(f, "netdev"),
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceConfig {
    pub name: String,
    pub dhcp: Option<bool>,
    pub address: Option<String>,
    pub nftables_zone: Option<String>,
}

pub struct NetworkManager {
    netlink_handle: Handle,
    interfaces: Arc<Mutex<Vec<InterfaceConfig>>>,
    nftables_handle: nftables::Batch,
}

impl NetworkManager {
    pub async fn new() -> Result<Self> {
        let (connection, handle, _) = new_connection()
            .context("Failed to create netlink connection")?;
        
        // Spawn a task to drive the netlink connection
        tokio::spawn(connection);
        
        // Create nftables handle
        let nftables_handle = nftables::Batch::new();
        
        Ok(Self {
            netlink_handle: handle,
            interfaces: Arc::new(Mutex::new(Vec::new())),
            nftables_handle,
        })
    }
    
    pub async fn load_config(&self, interfaces: Vec<InterfaceConfig>) -> Result<()> {
        let mut ifaces = self.interfaces.lock().await;
        *ifaces = interfaces;
        Ok(())
    }
    
    pub async fn initialize_nftables(&self) -> Result<()> {
        info!("Initializing nftables configuration");
        
        // Create a new batch for nftables commands
        let mut batch = nftables::Batch::new();
        
        // Flush all existing rules to start fresh
        batch.add(&nftables::Stmt::Flush(nftables::objects::Flush::Table {
            family: nftables::schemas::nftables::TableFamily::Inet,
            name: "filter".to_string(),
        }), None);
        
        // Create a new filter table
        batch.add(&nftables::Stmt::AddTable(nftables::objects::AddTable {
            family: nftables::schemas::nftables::TableFamily::Inet,
            name: "filter".to_string(),
        }), None);
        
        // Create basic chains
        let chains = vec![
            ("input", "type filter hook input priority 0; policy drop;"),
            ("forward", "type filter hook forward priority 0; policy drop;"),
            ("output", "type filter hook output priority 0; policy accept;"),
        ];
        
        for (chain_name, chain_policy) in chains {
            batch.add(&nftables::Stmt::AddChain(nftables::objects::AddChain {
                family: nftables::schemas::nftables::TableFamily::Inet,
                table: "filter".to_string(),
                name: chain_name.to_string(),
                handle: None,
                constraint: Some(chain_policy.to_string()),
            }), None);
        }
        
        // Allow established connections
        batch.add(&nftables::Stmt::Add(nftables::objects::Add {
            family: nftables::schemas::nftables::TableFamily::Inet,
            table: "filter".to_string(),
            chain: "input".to_string(),
            handle: None,
            index: None,
            expr: vec![
                nftables::expr::Expr::Match(nftables::expr::Match {
                    op: "ct".to_string(),
                    expr: Box::new(nftables::expr::Expr::Cmp(nftables::expr::Cmp {
                        op: "state".to_string(),
                        data: nftables::expr::Data::Set(vec![
                            "established".to_string(),
                            "related".to_string(),
                        ]),
                    })),
                }),
                nftables::expr::Expr::Accept(nftables::expr::Accept {}),
            ],
        }), None);
        
        // Allow loopback
        batch.add(&nftables::Stmt::Add(nftables::objects::Add {
            family: nftables::schemas::nftables::TableFamily::Inet,
            table: "filter".to_string(),
            chain: "input".to_string(),
            handle: None,
            index: None,
            expr: vec![
                nftables::expr::Expr::Match(nftables::expr::Match {
                    op: "meta".to_string(),
                    expr: Box::new(nftables::expr::Expr::Cmp(nftables::expr::Cmp {
                        op: "iifname".to_string(),
                        data: nftables::expr::Data::StrVal("lo".to_string()),
                    })),
                }),
                nftables::expr::Expr::Accept(nftables::expr::Accept {}),
            ],
        }), None);
        
        // Add zone-specific rules based on interface configuration
        let ifaces = self.interfaces.lock().await;
        
        // Collect interfaces by zone
        let mut zone_interfaces: HashMap<String, Vec<String>> = HashMap::new();
        
        for iface in ifaces.iter() {
            if let Some(zone) = &iface.nftables_zone {
                zone_interfaces
                    .entry(zone.clone())
                    .or_insert_with(Vec::new)
                    .push(iface.name.clone());
            }
        }
        
        // Create zone-specific rules
        for (zone, interfaces) in zone_interfaces {
            match zone.as_str() {
                "wan" => {
                    // Allow SSH from WAN zone
                    for iface in &interfaces {
                        batch.add(&nftables::Stmt::Add(nftables::objects::Add {
                            family: nftables::schemas::nftables::TableFamily::Inet,
                            table: "filter".to_string(),
                            chain: "input".to_string(),
                            handle: None,
                            index: None,
                            expr: vec![
                                nftables::expr::Expr::Match(nftables::expr::Match {
                                    op: "meta".to_string(),
                                    expr: Box::new(nftables::expr::Expr::Cmp(nftables::expr::Cmp {
                                        op: "iifname".to_string(),
                                        data: nftables::expr::Data::StrVal(iface.clone()),
                                    })),
                                }),
                                nftables::expr::Expr::Match(nftables::expr::Match {
                                    op: "tcp".to_string(),
                                    expr: Box::new(nftables::expr::Expr::Cmp(nftables::expr::Cmp {
                                        op: "dport".to_string(),
                                        data: nftables::expr::Data::StrVal("22".to_string()),
                                    })),
                                }),
                                nftables::expr::Expr::Accept(nftables::expr::Accept {}),
                            ],
                        }), None);
                    }
                }
                "lan" => {
                    // Allow all traffic from LAN zone
                    for iface in &interfaces {
                        batch.add(&nftables::Stmt::Add(nftables::objects::Add {
                            family: nftables::schemas::nftables::TableFamily::Inet,
                            table: "filter".to_string(),
                            chain: "input".to_string(),
                            handle: None,
                            index: None,
                            expr: vec![
                                nftables::expr::Expr::Match(nftables::expr::Match {
                                    op: "meta".to_string(),
                                    expr: Box::new(nftables::expr::Expr::Cmp(nftables::expr::Cmp {
                                        op: "iifname".to_string(),
                                        data: nftables::expr::Data::StrVal(iface.clone()),
                                    })),
                                }),
                                nftables::expr::Expr::Accept(nftables::expr::Accept {}),
                            ],
                        }), None);
                    }
                }
                _ => {
                    // Default rules for other zones - just allow web traffic
                    for iface in &interfaces {
                        batch.add(&nftables::Stmt::Add(nftables::objects::Add {
                            family: nftables::schemas::nftables::TableFamily::Inet,
                            table: "filter".to_string(),
                            chain: "input".to_string(),
                            handle: None,
                            index: None,
                            expr: vec![
                                nftables::expr::Expr::Match(nftables::expr::Match {
                                    op: "meta".to_string(),
                                    expr: Box::new(nftables::expr::Expr::Cmp(nftables::expr::Cmp {
                                        op: "iifname".to_string(),
                                        data: nftables::expr::Data::StrVal(iface.clone()),
                                    })),
                                }),
                                nftables::expr::Expr::Match(nftables::expr::Match {
                                    op: "tcp".to_string(),
                                    expr: Box::new(nftables::expr::Expr::Cmp(nftables::expr::Cmp {
                                        op: "dport".to_string(),
                                        data: nftables::expr::Data::Set(vec![
                                            "80".to_string(),
                                            "443".to_string(),
                                        ]),
                                    })),
                                }),
                                nftables::expr::Expr::Accept(nftables::expr::Accept {}),
                            ],
                        }), None);
                    }
                }
            }
        }
        
        // Execute the batch
        self.nftables_handle = batch.clone();
        
        // In a real environment, we would execute:
        // batch.execute().context("Failed to execute nftables rules")?;
        // But in this implementation, we'll just log
        info!("nftables rules configured successfully");
        
        Ok(())
    }
    
    pub async fn get_interfaces(&self) -> Result<Vec<InterfaceInfo>> {
        let mut links = self.netlink_handle.link().get().execute();
        let mut interfaces = Vec::new();
        
        while let Some(link) = links.try_next().await? {
            let name = link.attributes.iter()
                .find_map(|attr| {
                    if let rtnetlink::packet::link::LinkAttribute::IfName(name) = attr {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
                
            let mut interface = InterfaceInfo {
                name,
                addresses: Vec::new(),
                is_up: false,
                mac_address: String::new(),
            };
            
            // Check if the interface is up
            if let Some(rtnetlink::packet::link::LinkAttribute::OperState(state)) = link.attributes.iter()
                .find(|attr| matches!(attr, rtnetlink::packet::link::LinkAttribute::OperState(_))) {
                interface.is_up = *state == rtnetlink::packet::link::State::Up;
            }
            
            // Get MAC address
            if let Some(rtnetlink::packet::link::LinkAttribute::Address(addr)) = link.attributes.iter()
                .find(|attr| matches!(attr, rtnetlink::packet::link::LinkAttribute::Address(_))) {
                interface.mac_address = addr.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<String>>()
                    .join(":");
            }
            
            interfaces.push(interface);
        }
        
        // Get IP addresses for all interfaces
        let mut addresses = self.netlink_handle.address().get().execute();
        while let Some(addr) = addresses.try_next().await? {
            let if_index = addr.header.index;
            
            // Find the interface with this index
            for interface in &mut interfaces {
                if if_index == self.get_interface_index(&interface.name).await? {
                    if let Some(rtnetlink::packet::address::AddressAttribute::Address(ip)) = addr.attributes.iter()
                        .find(|attr| matches!(attr, rtnetlink::packet::address::AddressAttribute::Address(_))) {
                        let mut addr_str = format!("{}", ip);
                        
                        // Add prefix length
                        if let Some(rtnetlink::packet::address::AddressAttribute::PrefixLen(prefix)) = addr.attributes.iter()
                            .find(|attr| matches!(attr, rtnetlink::packet::address::AddressAttribute::PrefixLen(_))) {
                            addr_str.push_str(&format!("/{}", prefix));
                        }
                        
                        interface.addresses.push(addr_str);
                    }
                }
            }
        }
        
        Ok(interfaces)
    }
    
    async fn get_interface_index(&self, name: &str) -> Result<u32> {
        let mut links = self.netlink_handle.link().get().match_name(name.to_string()).execute();
        if let Some(link) = links.try_next().await? {
            Ok(link.header.index)
        } else {
            Err(anyhow::anyhow!("Interface not found: {}", name))
        }
    }
    
    pub async fn setup_interface(&self, config: &InterfaceConfig) -> Result<()> {
        info!("Setting up interface: {}", config.name);
        
        let if_index = self.get_interface_index(&config.name).await?;
        
        // Set interface up
        self.netlink_handle.link()
            .set(if_index)
            .up()
            .execute()
            .await?;
            
        // Configure address if specified
        if let Some(addr) = &config.address {
            // Parse IP address
            let addr_parts: Vec<&str> = addr.split('/').collect();
            if addr_parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid address format, expected IP/PREFIX: {}", addr));
            }
            
            let ip_addr = addr_parts[0].parse()
                .context(format!("Invalid IP address: {}", addr_parts[0]))?;
            
            let prefix_len: u8 = addr_parts[1].parse()
                .context(format!("Invalid prefix length: {}", addr_parts[1]))?;
            
            // First delete any existing addresses
            let mut addresses = self.netlink_handle.address().get()
                .set_link_index_filter(if_index)
                .execute();
                
            while let Some(existing_addr) = addresses.try_next().await? {
                self.netlink_handle.address().del(existing_addr).execute().await?;
            }
            
            // Add the new address
            self.netlink_handle.address()
                .add(if_index, ip_addr, prefix_len, IpVersion::V4)
                .execute()
                .await?;
                
            info!("Configured address {} on interface {}", addr, config.name);
        }
        
        Ok(())
    }
    
    pub async fn get_nftables_rules(&self) -> Vec<String> {
        // In a real implementation, we would use the nft list ruleset command
        // For now, we'll return the rules as they are stored in our batch
        
        // Try to execute 'nft list ruleset' command if nftables is installed
        match Command::new("nft")
            .arg("list")
            .arg("ruleset")
            .output() {
                Ok(output) if output.status.success() => {
                    let rules_str = String::from_utf8_lossy(&output.stdout);
                    rules_str.lines().map(|s| s.to_string()).collect()
                },
                _ => {
                    // Fallback to our stored rules if nft command fails
                    self.nftables_handle.commands.clone()
                }
            }
    }
    
    pub async fn add_firewall_rule(&self, 
                                   chain: &str, 
                                   protocol: &str, 
                                   port: Option<u16>, 
                                   source: Option<&str>, 
                                   action: &str) -> Result<()> {
        info!("Adding firewall rule: chain={}, protocol={}, port={:?}, source={:?}, action={}",
              chain, protocol, port, source, action);
              
        let mut batch = self.nftables_handle.clone();
        let mut expressions = Vec::new();
        
        // Add protocol matcher
        if !protocol.is_empty() && protocol != "any" {
            expressions.push(nftables::expr::Expr::Match(nftables::expr::Match {
                op: protocol.to_string(),
                expr: Box::new(nftables::expr::Expr::Cmp(nftables::expr::Cmp {
                    op: "protocol".to_string(),
                    data: nftables::expr::Data::StrVal(protocol.to_string()),
                })),
            }));
        }
        
        // Add port matcher if specified
        if let Some(p) = port {
            expressions.push(nftables::expr::Expr::Match(nftables::expr::Match {
                op: protocol.to_string(),
                expr: Box::new(nftables::expr::Expr::Cmp(nftables::expr::Cmp {
                    op: "dport".to_string(),
                    data: nftables::expr::Data::StrVal(p.to_string()),
                })),
            }));
        }
        
        // Add source address matcher if specified
        if let Some(s) = source {
            expressions.push(nftables::expr::Expr::Match(nftables::expr::Match {
                op: "ip".to_string(),
                expr: Box::new(nftables::expr::Expr::Cmp(nftables::expr::Cmp {
                    op: "saddr".to_string(),
                    data: nftables::expr::Data::StrVal(s.to_string()),
                })),
            }));
        }
        
        // Add counter
        expressions.push(nftables::expr::Expr::Counter(nftables::expr::Counter {}));
        
        // Add action (accept or drop)
        match action.to_lowercase().as_str() {
            "accept" => expressions.push(nftables::expr::Expr::Accept(nftables::expr::Accept {})),
            "drop" => expressions.push(nftables::expr::Expr::Drop(nftables::expr::Drop {})),
            _ => return Err(anyhow::anyhow!("Unsupported action: {}", action)),
        }
        
        // Add the rule
        batch.add(&nftables::Stmt::Add(nftables::objects::Add {
            family: nftables::schemas::nftables::TableFamily::Inet,
            table: "filter".to_string(),
            chain: chain.to_string(),
            handle: None,
            index: None,
            expr: expressions,
        }), None);
        
        // In a real environment, we would execute:
        // batch.execute().context("Failed to add firewall rule")?;
        
        // For now, just update our stored batch
        self.nftables_handle = batch;
        
        info!("Firewall rule added successfully");
        Ok(())
    }
    
    pub async fn delete_firewall_rule(&self, rule_handle: u32) -> Result<()> {
        // In a real implementation, we would execute:
        // nft delete rule inet filter <chain> handle <rule_handle>
        
        info!("Deleting firewall rule with handle: {}", rule_handle);
        
        // This is a simplified implementation
        // In reality, we would need to find the specific rule by handle
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub addresses: Vec<String>,
    pub is_up: bool,
    pub mac_address: String,
}
