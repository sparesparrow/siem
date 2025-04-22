<img src="https://r2cdn.perplexity.ai/pplx-full-logo-primary-dark%402x.png" class="logo" width="120"/>

# Aktualizace vizualizační části SIEM systému pro ZOO Brno

Předložená aktualizace kódové základny výrazně rozšiřuje vizualizační možnosti SIEM systému pro ZOO Brno. Tato aktualizace je velmi vhodná, protože přináší komplexní sadu grafických prvků, které umožní jedinému IT administrátorovi efektivně spravovat síť se 100 zaměstnanci. Implementace těchto vizualizací by měla být provedena v souladu s již navrženou architekturou a s ohledem na bezpečnostní požadavky vyplývající z vyhlášky č. 82/2018 Sb.

## Implementace dashboardu a vizualizací

Pro implementaci navržených vizualizací doporučuji využít kombinaci Leptos frameworku (pro reaktivní UI komponenty) a knihovny Plotly (pro grafy a vizualizace). Následující kód ukazuje, jak implementovat hlavní dashboard s využitím WebSocket pro real-time aktualizace dat.

```rust
// web/src/pages/dashboard.rs
use leptos::*;
use leptos_meta::*;
use web_sys::{HtmlCanvasElement, HtmlDivElement};
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

// Datové struktury pro dashboard
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardStats {
    pub active_alerts: u32,
    pub open_tickets: u32,
    pub running_scripts: u32,
    pub network_health: u8,
    pub traffic_history: Vec&lt;TrafficPoint&gt;,
    pub ticket_distribution: TicketDistribution,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrafficPoint {
    pub timestamp: String,
    pub incoming_mbps: f64,
    pub outgoing_mbps: f64,
    pub is_anomaly: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TicketDistribution {
    pub new: u32,
    pub in_progress: u32,
    pub resolved: u32,
}

#[component]
pub fn Dashboard() -&gt; impl IntoView {
    // Získání dat z API pomocí resource
    let stats_resource = create_resource(
        || (), 
        |_| async move {
            let resp = reqwest::get("/api/dashboard/stats")
                .await
                .unwrap()
                .json::&lt;DashboardStats&gt;()
                .await
                .unwrap();
            resp
        }
    );
    
    // Reference na DOM elementy pro grafy
    let traffic_chart_ref = create_node_ref::&lt;HtmlDivElement&gt;();
    let ticket_pie_ref = create_node_ref::&lt;HtmlDivElement&gt;();
    
    // WebSocket pro real-time aktualizace
    let (ws_data, set_ws_data) = create_signal(None::&lt;DashboardStats&gt;);
    
    // Inicializace WebSocket
    create_effect(move |_| {
        let ws = WebSocket::new("wss://siem.zoobrno.cz/api/ws/dashboard").unwrap();
        
        let callback = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            let data: DashboardStats = serde_json::from_str(&amp;e.data().as_string().unwrap()).unwrap();
            set_ws_data.set(Some(data));
        }) as Box&lt;dyn FnMut(_)&gt;);
        
        ws.set_onmessage(Some(callback.as_ref().unchecked_ref()));
        callback.forget(); // Předejít uvolnění closure
    });
    
    // Vykreslení grafů po načtení dat
    create_effect(move |_| {
        if let Some(stats) = stats_resource.get().or_else(|| ws_data.get()) {
            if let Some(div) = traffic_chart_ref.get() {
                // Kód pro vykreslení grafu síťového provozu pomocí Plotly
                let mut plot = Plot::new();
                
                let timestamps: Vec&lt;String&gt; = stats.traffic_history.iter()
                    .map(|p| p.timestamp.clone())
                    .collect();
                
                let incoming: Vec&lt;f64&gt; = stats.traffic_history.iter()
                    .map(|p| p.incoming_mbps)
                    .collect();
                
                let outgoing: Vec&lt;f64&gt; = stats.traffic_history.iter()
                    .map(|p| p.outgoing_mbps)
                    .collect();
                
                let incoming_trace = Scatter::new(timestamps.clone(), incoming)
                    .name("Příchozí provoz (Mbps)")
                    .line(plotly::common::Line::new().color("blue"));
                
                let outgoing_trace = Scatter::new(timestamps, outgoing)
                    .name("Odchozí provoz (Mbps)")
                    .line(plotly::common::Line::new().color("orange"));
                
                plot.add_trace(incoming_trace);
                plot.add_trace(outgoing_trace);
                
                let layout = Layout::new()
                    .title("Síťový provoz za posledních 24 hodin")
                    .height(300)
                    .margin(plotly::layout::Margin::new().top(30).bottom(30).left(50).right(20));
                
                plot.set_layout(layout);
                plot.show();
            }
            
            if let Some(div) = ticket_pie_ref.get() {
                // Kód pro vykreslení koláčového grafu distribuce tiketů
                let mut plot = Plot::new();
                
                let labels = vec!["Nové", "V řešení", "Vyřešené"];
                let values = vec![
                    stats.ticket_distribution.new,
                    stats.ticket_distribution.in_progress,
                    stats.ticket_distribution.resolved
                ];
                let colors = vec!["#3498db", "#f1c40f", "#2ecc71"];
                
                let pie = Pie::new(labels, values)
                    .marker(plotly::common::Marker::new().colors(colors))
                    .hole(0.4);
                
                plot.add_trace(pie);
                
                let layout = Layout::new()
                    .title("Distribuce tiketů")
                    .height(250)
                    .margin(plotly::layout::Margin::new().top(30).bottom(10).left(10).right(10));
                
                plot.set_layout(layout);
                plot.show();
            }
        }
    });
    
    view! {
        <div>
            <h1>"SIEM Dashboard"</h1>
            
            // Statistické karty
            <div>
                <div>
                    <div>&lt;i class="fas fa-bell"&gt;&lt;/i&gt;</div>
                    <div>
                        <div>
                            {move || stats_resource.get().map(|s| s.active_alerts.to_string()).unwrap_or_else(|| "...".to_string())}
                        </div>
                        <div>"Aktivní alerty"</div>
                    </div>
                </div>
                
                <div>
                    <div>&lt;i class="fas fa-ticket-alt"&gt;&lt;/i&gt;</div>
                    <div>
                        <div>
                            {move || stats_resource.get().map(|s| s.open_tickets.to_string()).unwrap_or_else(|| "...".to_string())}
                        </div>
                        <div>"Otevřené tikety"</div>
                    </div>
                </div>
                
                <div>
                    <div>&lt;i class="fas fa-code"&gt;&lt;/i&gt;</div>
                    <div>
                        <div>
                            {move || stats_resource.get().map(|s| s.running_scripts.to_string()).unwrap_or_else(|| "...".to_string())}
                        </div>
                        <div>"Běžící skripty"</div>
                    </div>
                </div>
                
                <div>
                    <div>&lt;i class="fas fa-heartbeat"&gt;&lt;/i&gt;</div>
                    <div>
                        <div>
                            {move || stats_resource.get().map(|s| format!("{}%", s.network_health)).unwrap_or_else(|| "...".to_string())}
                        </div>
                        <div>"Zdraví sítě"</div>
                    </div>
                </div>
            </div>
            
            // Grafy
            <div>
                <div>
                    <h2>"Síťový provoz"</h2>
                    <div></div>
                </div>
                
                <div>
                    <h2>"Distribuce tiketů"</h2>
                    <div></div>
                </div>
            </div>
        </div>
    }
}
```


## Implementace monitorování sítě

Následující kód implementuje síťový monitoring s interaktivním grafem a tabulkou stavu zařízení:

```rust
// web/src/pages/network_monitoring.rs
use leptos::*;
use leptos_meta::*;
use web_sys::HtmlDivElement;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkDevice {
    pub id: String,
    pub name: String,
    pub ip_address: String,
    pub status: DeviceStatus,
    pub last_seen: String,
    pub uptime: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DeviceStatus {
    Online,
    Offline,
    Warning,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConnection {
    pub source: String,
    pub target: String,
    pub bandwidth: f64,
    pub status: ConnectionStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    Active,
    Inactive,
    Degraded,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkData {
    pub devices: Vec&lt;NetworkDevice&gt;,
    pub connections: Vec&lt;NetworkConnection&gt;,
    pub traffic_data: Vec&lt;TrafficPoint&gt;,
}

#[component]
pub fn NetworkMonitoring() -&gt; impl IntoView {
    // Získání dat z API
    let network_data = create_resource(
        || (), 
        |_| async move {
            let resp = reqwest::get("/api/network/status")
                .await
                .unwrap()
                .json::&lt;NetworkData&gt;()
                .await
                .unwrap();
            resp
        }
    );
    
    // Reference na DOM elementy pro grafy
    let network_graph_ref = create_node_ref::&lt;HtmlDivElement&gt;();
    let traffic_chart_ref = create_node_ref::&lt;HtmlDivElement&gt;();
    
    // Vykreslení síťového grafu
    create_effect(move |_| {
        if let Some(data) = network_data.get() {
            if let Some(div) = network_graph_ref.get() {
                // Kód pro vykreslení síťového grafu pomocí Plotly nebo D3.js
                // Zde by byl kód pro vykreslení interaktivního grafu sítě
                // s uzly (zařízení) a hranami (spojení)
            }
            
            if let Some(div) = traffic_chart_ref.get() {
                // Kód pro vykreslení grafu síťového provozu
                let mut plot = Plot::new();
                
                let timestamps: Vec&lt;String&gt; = data.traffic_data.iter()
                    .map(|p| p.timestamp.clone())
                    .collect();
                
                let incoming: Vec&lt;f64&gt; = data.traffic_data.iter()
                    .map(|p| p.incoming_mbps)
                    .collect();
                
                let outgoing: Vec&lt;f64&gt; = data.traffic_data.iter()
                    .map(|p| p.outgoing_mbps)
                    .collect();
                
                let incoming_trace = Scatter::new(timestamps.clone(), incoming)
                    .name("Příchozí provoz (Mbps)")
                    .line(plotly::common::Line::new().color("blue"));
                
                let outgoing_trace = Scatter::new(timestamps, outgoing)
                    .name("Odchozí provoz (Mbps)")
                    .line(plotly::common::Line::new().color("orange"));
                
                plot.add_trace(incoming_trace);
                plot.add_trace(outgoing_trace);
                
                let layout = Layout::new()
                    .title("Síťový provoz v reálném čase")
                    .height(300)
                    .margin(plotly::layout::Margin::new().top(30).bottom(30).left(50).right(20));
                
                plot.set_layout(layout);
                plot.show();
            }
        }
    });
    
    view! {
        <div>
            <h1>"Monitorování sítě"</h1>
            
            <div>
                <div>
                    <h2>"Síťový graf"</h2>
                    <div>
                        &lt;button class="zoom-in"&gt;"Přiblížit"&lt;/button&gt;
                        &lt;button class="zoom-out"&gt;"Oddálit"&lt;/button&gt;
                        &lt;select class="filter"&gt;
                            &lt;option value="all"&gt;"Všechna zařízení"&lt;/option&gt;
                            &lt;option value="active"&gt;"Pouze aktivní"&lt;/option&gt;
                            &lt;option value="servers"&gt;"Pouze servery"&lt;/option&gt;
                        &lt;/select&gt;
                    </div>
                    <div></div>
                    <div>
                        <div><span></span>"Online"</div>
                        <div><span></span>"Offline"</div>
                        <div><span></span>"Varování"</div>
                    </div>
                </div>
                
                <div>
                    <h2>"Síťový provoz"</h2>
                    <div></div>
                </div>
            </div>
            
            <div>
                <h2>"Stav zařízení"</h2>
                &lt;table&gt;
                    &lt;thead&gt;
                        &lt;tr&gt;
                            &lt;th&gt;"Název zařízení"&lt;/th&gt;
                            &lt;th&gt;"IP adresa"&lt;/th&gt;
                            &lt;th&gt;"Stav"&lt;/th&gt;
                            &lt;th&gt;"Naposledy viděno"&lt;/th&gt;
                            &lt;th&gt;"Uptime"&lt;/th&gt;
                            &lt;th&gt;"Akce"&lt;/th&gt;
                        &lt;/tr&gt;
                    &lt;/thead&gt;
                    &lt;tbody&gt;
                        {move || network_data.get().map(|data| {
                            data.devices.iter().map(|device| {
                                let status_class = match device.status {
                                    DeviceStatus::Online =&gt; "status-online",
                                    DeviceStatus::Offline =&gt; "status-offline",
                                    DeviceStatus::Warning =&gt; "status-warning",
                                };
                                
                                view! {
                                    &lt;tr&gt;
                                        &lt;td&gt;{&amp;device.name}&lt;/td&gt;
                                        &lt;td&gt;{&amp;device.ip_address}&lt;/td&gt;
                                        &lt;td class={status_class}&gt;
                                            <span></span>
                                            {match device.status {
                                                DeviceStatus::Online =&gt; "Online",
                                                DeviceStatus::Offline =&gt; "Offline",
                                                DeviceStatus::Warning =&gt; "Varování",
                                            }}
                                        &lt;/td&gt;
                                        &lt;td&gt;{&amp;device.last_seen}&lt;/td&gt;
                                        &lt;td&gt;{format_uptime(device.uptime)}&lt;/td&gt;
                                        &lt;td&gt;
                                            &lt;button class="action-button"&gt;"Detaily"&lt;/button&gt;
                                            &lt;button class="action-button"&gt;"Ping"&lt;/button&gt;
                                        &lt;/td&gt;
                                    &lt;/tr&gt;
                                }
                            }).collect_view()
                        }).unwrap_or_else(|| view! { &lt;tr&gt;&lt;td colspan="6"&gt;"Načítání dat..."&lt;/td&gt;&lt;/tr&gt; })}
                    &lt;/tbody&gt;
                &lt;/table&gt;
            </div>
        </div>
    }
}

fn format_uptime(uptime_seconds: u64) -&gt; String {
    let days = uptime_seconds / 86400;
    let hours = (uptime_seconds % 86400) / 3600;
    let minutes = (uptime_seconds % 3600) / 60;
    
    if days &gt; 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours &gt; 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}
```


## Implementace správy tiskáren

Následující kód implementuje správu tiskáren s kartami, grafy úrovní toneru a seznamem chyb:

```rust
// web/src/pages/printer_management.rs
use leptos::*;
use leptos_meta::*;
use web_sys::HtmlDivElement;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Printer {
    pub id: String,
    pub name: String,
    pub status: PrinterStatus,
    pub toner_level: u8,
    pub error_message: Option&lt;String&gt;,
    pub location: String,
    pub last_updated: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PrinterStatus {
    Online,
    Offline,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrinterError {
    pub printer_id: String,
    pub printer_name: String,
    pub error_type: String,
    pub timestamp: String,
    pub severity: ErrorSeverity,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Ord, PartialOrd, Eq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Critical,
}

#[component]
pub fn PrinterManagement() -&gt; impl IntoView {
    // Získání dat z API
    let printers_data = create_resource(
        || (), 
        |_| async move {
            let resp = reqwest::get("/api/printers")
                .await
                .unwrap()
                .json::&lt;Vec&lt;Printer&gt;&gt;()
                .await
                .unwrap();
            resp
        }
    );
    
    let printer_errors = create_resource(
        || (), 
        |_| async move {
            let resp = reqwest::get("/api/printers/errors")
                .await
                .unwrap()
                .json::&lt;Vec&lt;PrinterError&gt;&gt;()
                .await
                .unwrap();
            resp
        }
    );
    
    // Reference na DOM element pro graf úrovní toneru
    let toner_chart_ref = create_node_ref::&lt;HtmlDivElement&gt;();
    
    // Vykreslení grafu úrovní toneru
    create_effect(move |_| {
        if let Some(printers) = printers_data.get() {
            if let Some(div) = toner_chart_ref.get() {
                // Kód pro vykreslení horizontálního sloupcového grafu úrovní toneru
                let mut plot = Plot::new();
                
                let names: Vec&lt;String&gt; = printers.iter()
                    .map(|p| p.name.clone())
                    .collect();
                
                let toner_levels: Vec&lt;u8&gt; = printers.iter()
                    .map(|p| p.toner_level)
                    .collect();
                
                let colors: Vec&lt;&amp;str&gt; = toner_levels.iter()
                    .map(|&amp;level| {
                        if level &gt; 50 { "#2ecc71" }      // Zelená pro &gt;50%
                        else if level &gt; 20 { "#f1c40f" } // Žlutá pro 20-50%
                        else { "#e74c3c" }               // Červená pro &lt;20%
                    })
                    .collect();
                
                let bar = Bar::new(names, toner_levels)
                    .orientation(plotly::common::Orientation::Horizontal)
                    .marker(plotly::common::Marker::new().colors(colors));
                
                plot.add_trace(bar);
                
                let layout = Layout::new()
                    .title("Úrovně toneru")
                    .height(400)
                    .margin(plotly::layout::Margin::new().top(30).bottom(30).left(100).right(20))
                    .x_axis(plotly::layout::Axis::new().title("Úroveň toneru (%)").range(vec![0, 100]));
                
                plot.set_layout(layout);
                plot.show();
            }
        }
    });
    
    view! {
        <div>
            <h1>"Správa tiskáren"</h1>
            
            <div>
                {move || printers_data.get().map(|printers| {
                    printers.iter().map(|printer| {
                        let status_class = match printer.status {
                            PrinterStatus::Online =&gt; "status-online",
                            PrinterStatus::Offline =&gt; "status-offline",
                            PrinterStatus::Error =&gt; "status-error",
                        };
                        
                        let toner_class = if printer.toner_level &gt; 50 {
                            "toner-high"
                        } else if printer.toner_level &gt; 20 {
                            "toner-medium"
                        } else {
                            "toner-low"
                        };
                        
                        view! {
                            <div>
                                <div>
                                    <h3>{&amp;printer.name}</h3>
                                    <div>
                                        {match printer.status {
                                            PrinterStatus::Online =&gt; "Online",
                                            PrinterStatus::Offline =&gt; "Offline",
                                            PrinterStatus::Error =&gt; "Chyba",
                                        }}
                                    </div>
                                </div>
                                <div>
                                    <div>{&amp;printer.location}</div>
                                    <div>
                                        <span>"Úroveň toneru: "</span>
                                        <div>
                                            <div></div>
                                        </div>
                                        <span>{format!("{}%", printer.toner_level)}</span>
                                    </div>
                                    {if let Some(error) = &amp;printer.error_message {
                                        view! {
                                            <div>{error}</div>
                                        }
                                    } else {
                                        view! { <div>"Žádné chyby"</div> }
                                    }}
                                </div>
                                <div>
                                    <div>{format!("Aktualizováno: {}", printer.last_updated)}</div>
                                    <div>
                                        &lt;button class="action-button"&gt;"Detaily"&lt;/button&gt;
                                        &lt;button class="action-button"&gt;"Test"&lt;/button&gt;
                                    </div>
                                </div>
                            </div>
                        }
                    }).collect_view()
                }).unwrap_or_else(|| view! { <div>"Načítání tiskáren..."</div> })}
            </div>
            
            <div>
                <h2>"Úrovně toneru"</h2>
                <div></div>
            </div>
            
            <div>
                <h2>"Seznam chyb tiskáren"</h2>
                &lt;table&gt;
                    &lt;thead&gt;
                        &lt;tr&gt;
                            &lt;th&gt;"Tiskárna"&lt;/th&gt;
                            &lt;th&gt;"Typ chyby"&lt;/th&gt;
                            &lt;th&gt;"Závažnost"&lt;/th&gt;
                            &lt;th&gt;"Čas"&lt;/th&gt;
                            &lt;th&gt;"Akce"&lt;/th&gt;
                        &lt;/tr&gt;
                    &lt;/thead&gt;
                    &lt;tbody&gt;
                        {move || printer_errors.get().map(|errors| {
                            // Seřazení chyb podle závažnosti (kritické nahoře)
                            let mut sorted_errors = errors.clone();
                            sorted_errors.sort_by(|a, b| b.severity.cmp(&amp;a.severity));
                            
                            sorted_errors.iter().map(|error| {
                                let severity_class = match error.severity {
                                    ErrorSeverity::Info =&gt; "severity-info",
                                    ErrorSeverity::Warning =&gt; "severity-warning",
                                    ErrorSeverity::Critical =&gt; "severity-critical",
                                };
                                
                                view! {
                                    &lt;tr&gt;
                                        &lt;td&gt;{&amp;error.printer_name}&lt;/td&gt;
                                        &lt;td&gt;{&amp;error.error_type}&lt;/td&gt;
                                        &lt;td class={severity_class}&gt;
                                            {match error.severity {
                                                ErrorSeverity::Info =&gt; "Informace",
                                                ErrorSeverity::Warning =&gt; "Varování",
                                                ErrorSeverity::Critical =&gt; "Kritická",
                                            }}
                                        &lt;/td&gt;
                                        &lt;td&gt;{&amp;error.timestamp}&lt;/td&gt;
                                        &lt;td&gt;
                                            &lt;button class="action-button"&gt;"Vyřešit"&lt;/button&gt;
                                        &lt;/td&gt;
                                    &lt;/tr&gt;
                                }
                            }).collect_view()
                        }).unwrap_or_else(|| view! { &lt;tr&gt;&lt;td colspan="5"&gt;"Žádné chyby"&lt;/td&gt;&lt;/tr&gt; })}
                    &lt;/tbody&gt;
                &lt;/table&gt;
            </div>
        </div>
    }
}
```


## Implementace tiketového systému

Následující kód implementuje tiketový systém s Kanban boardem a grafy:

```rust
// web/src/pages/ticketing.rs
use leptos::*;
use leptos_meta::*;
use web_sys::HtmlDivElement;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub category: String,
    pub submitter: String,
    pub assigned_to: Option&lt;String&gt;,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TicketStatus {
    New,
    InProgress,
    Resolved,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[component]
pub fn TicketingSystem() -&gt; impl IntoView {
    // Získání dat z API
    let tickets_data = create_resource(
        || (), 
        |_| async move {
            let resp = reqwest::get("/api/tickets")
                .await
                .unwrap()
                .json::&lt;Vec&lt;Ticket&gt;&gt;()
                .await
                .unwrap();
            resp
        }
    );
    
    // Reference na DOM elementy pro grafy
    let priority_chart_ref = create_node_ref::&lt;HtmlDivElement&gt;();
    let category_chart_ref = create_node_ref::&lt;HtmlDivElement&gt;();
    
    // Vykreslení grafů
    create_effect(move |_| {
        if let Some(tickets) = tickets_data.get() {
            if let Some(div) = priority_chart_ref.get() {
                // Kód pro vykreslení koláčového grafu priorit tiketů
                let mut plot = Plot::new();
                
                // Počítání tiketů podle priority
                let mut low_count = 0;
                let mut medium_count = 0;
                let mut high_count = 0;
                let mut critical_count = 0;
                
                for ticket in &amp;tickets {
                    match ticket.priority {
                        TicketPriority::Low =&gt; low_count += 1,
                        TicketPriority::Medium =&gt; medium_count += 1,
                        TicketPriority::High =&gt; high_count += 1,
                        TicketPriority::Critical =&gt; critical_count += 1,
                    }
                }
                
                let labels = vec!["Nízká", "Střední", "Vysoká", "Kritická"];
                let values = vec![low_count, medium_count, high_count, critical_count];
                let colors = vec!["#2ecc71", "#f1c40f", "#e67e22", "#e74c3c"];
                
                let pie = Pie::new(labels, values)
                    .marker(plotly::common::Marker::new().colors(colors))
                    .hole(0.4);
                
                plot.add_trace(pie);
                
                let layout = Layout::new()
                    .title("Distribuce tiketů podle priority")
                    .height(300)
                    .margin(plotly::layout::Margin::new().top(30).bottom(10).left(10).right(10));
                
                plot.set_layout(layout);
                plot.show();
            }
            
            if let Some(div) = category_chart_ref.get() {
                // Kód pro vykreslení sloupcového grafu kategorií tiketů
                let mut plot = Plot::new();
                
                // Počítání tiketů podle kategorie
                let mut category_counts = std::collections::HashMap::new();
                
                for ticket in &amp;tickets {
                    *category_counts.entry(ticket.category.clone()).or_insert(0) += 1;
                }
                
                let categories: Vec&lt;String&gt; = category_counts.keys().cloned().collect();
                let counts: Vec&lt;u32&gt; = categories.iter().map(|c| *category_counts.get(c).unwrap()).collect();
                
                let bar = Bar::new(categories, counts);
                
                plot.add_trace(bar);
                
                let layout = Layout::new()
                    .title("Tikety podle kategorie")
                    .height(300)
                    .margin(plotly::layout::Margin::new().top(30).bottom(50).left(50).right(20));
                
                plot.set_layout(layout);
                plot.show();
            }
        }
    });
    
    view! {
        <div>
            <h1>"Tiketový systém"</h1>
            
            <div>
                &lt;button class="create-ticket"&gt;"Vytvořit nový tiket"&lt;/button&gt;
                <div>
                    &lt;select&gt;
                        &lt;option value="all"&gt;"Všechny tikety"&lt;/option&gt;
                        &lt;option value="my"&gt;"Moje tikety"&lt;/option&gt;
                        &lt;option value="unassigned"&gt;"Nepřiřazené"&lt;/option&gt;
                    &lt;/select&gt;
                </div>
            </div>
            
            <div>
                <div>
                    <h2>"Nové"</h2>
                    {move || tickets_data.get().map(|tickets| {
                        tickets.iter()
                            .filter(|t| t.status == TicketStatus::New)
                            .map(|ticket| {
                                let priority_class = match ticket.priority {
                                    TicketPriority::Low =&gt; "priority-low",
                                    TicketPriority::Medium =&gt; "priority-medium",
                                    TicketPriority::High =&gt; "priority-high",
                                    TicketPriority::Critical =&gt; "priority-critical",
                                };
                                
                                view! {
                                    <div>
                                        <div></div>
                                        <div>{&amp;ticket.title}</div>
                                        <div>
                                            <span>{format!("#{}", ticket.id)}</span>
                                            <span>{&amp;ticket.submitter}</span>
                                        </div>
                                        <div>{&amp;ticket.category}</div>
                                    </div>
                                }
                            })
                            .collect_view()
                    }).unwrap_or_else(|| view! { <div>"Načítání tiketů..."</div> })}
                </div>
                
                <div>
                    <h2>"V řešení"</h2>
                    {move || tickets_data.get().map(|tickets| {
                        tickets.iter()
                            .filter(|t| t.status == TicketStatus::InProgress)
                            .map(|ticket| {
                                let priority_class = match ticket.priority {
                                    TicketPriority::Low =&gt; "priority-low",
                                    TicketPriority::Medium =&gt; "priority-medium",
                                    TicketPriority::High =&gt; "priority-high",
                                    TicketPriority::Critical =&gt; "priority-critical",
                                };
                                
                                view! {
                                    <div>
                                        <div></div>
                                        <div>{&amp;ticket.title}</div>
                                        <div>
                                            <span>{format!("#{}", ticket.id)}</span>
                                            <span>{&amp;ticket.submitter}</span>
                                        </div>
                                        <div>{&amp;ticket.category}</div>
                                    </div>
                                }
                            })
                            .collect_view()
                    }).unwrap_or_else(|| view! { <div>"Načítání tiketů..."</div> })}
                </div>
                
                <div>
                    <h2>"Vyřešené"</h2>
                    {move || tickets_data.get().map(|tickets| {
                        tickets.iter()
                            .filter(|t| t.status == TicketStatus::Resolved)
                            .map(|ticket| {
                                let priority_class = match ticket.priority {
                                    TicketPriority::Low =&gt; "priority-low",
                                    TicketPriority::Medium =&gt; "priority-medium",
                                    TicketPriority::High =&gt; "priority-high",
                                    TicketPriority::Critical =&gt; "priority-critical",
                                };
                                
                                view! {
                                    <div>
                                        <div></div>
                                        <div>{&amp;ticket.title}</div>
                                        <div>
                                            <span>{format!("#{}", ticket.id)}</span>
                                            <span>{&amp;ticket.submitter}</span>
                                        </div>
                                        <div>{&amp;ticket.category}</div>
                                    </div>
                                }
                            })
                            .collect_view()
                    }).unwrap_or_else(|| view! { <div>"Načítání tiketů..."</div> })}
                </div>
            </div>
            
            <div>
                <div>
                    <h2>"Distribuce podle priority"</h2>
                    <div></div>
                </div>
                
                <div>
                    <h2>"Tikety podle kategorie"</h2>
                    <div></div>
                </div>
            </div>
        </div>
    }
}
```


## Doporučení pro implementaci

1. **Bezpečnost**: Vzhledem k požadavkům vyhlášky č. 82/2018 Sb. je nutné zajistit, aby všechny vizualizace respektovaly bezpečnostní principy:
    - Implementovat RBAC (Role-Based Access Control) pro všechny vizualizace
    - Zajistit, že citlivá data nejsou zobrazována neoprávněným uživatelům
    - Logovat všechny akce uživatelů pro účely auditu
2. **Výkon**: Pro zajištění plynulého chodu vizualizací i na slabších zařízeních:
    - Implementovat lazy loading pro grafy a vizualizace
    - Omezit množství dat přenášených přes WebSocket
    - Využít WebWorkers pro náročnější výpočty na straně klienta
3. **Uživatelská přívětivost**:
    - Zajistit konzistentní barevné schéma napříč všemi vizualizacemi
    - Implementovat responzivní design pro všechny komponenty
    - Přidat nápovědy a tooltipy pro méně technicky zdatné uživatele
4. **Penetrační testování**:
    - Před nasazením do produkce provést penetrační testování webového rozhraní
    - Zaměřit se na XSS, CSRF a další běžné zranitelnosti webových aplikací
    - Testovat odolnost proti útokům zaměřeným na WebSocket komunikaci

## Závěr

Navržená aktualizace kódové základny přináší komplexní sadu vizualizací, které výrazně zlepší použitelnost SIEM systému pro ZOO Brno. Implementace těchto vizualizací pomocí Leptos frameworku a Plotly knihovny zajistí vysoký výkon, bezpečnost a uživatelskou přívětivost. Díky reaktivnímu přístupu a real-time aktualizacím dat bude mít IT administrátor vždy aktuální přehled o stavu sítě, tiskáren, tiketů a skriptů, což mu umožní efektivně spravovat IT infrastrukturu pro 100 zaměstnanců.

Implementace těchto vizualizací by měla být provedena v souladu s již navrženou architekturou a s ohledem na bezpečnostní požadavky. Po implementaci doporučuji provést důkladné testování, včetně penetračního testování, aby byla zajištěna bezpečnost a spolehlivost celého systému.

<div style="text-align: center">⁂</div>

[^1]: https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/64960960/0361ba4e-0df0-47b1-8d4c-dceedcafa2dc/paste.txt

[^2]: https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/collection_5f13ca3e-bb49-43f2-a6ab-0c0a14fa36f7/35e73a79-dea4-45b7-97bd-a3d251faf07c/2022-03-07_Penetracni-testovani_v1.2.pdf

