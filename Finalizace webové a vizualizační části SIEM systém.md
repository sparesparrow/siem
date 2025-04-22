<img src="https://r2cdn.perplexity.ai/pplx-full-logo-primary-dark%402x.png" class="logo" width="120"/>

# Finalizace webové a vizualizační části SIEM systému pro ZOO Brno

Dokončení webové a vizualizační části SIEM systému pro ZOO Brno vyžaduje implementaci robustního, bezpečného a uživatelsky přívětivého řešení, které bude optimalizováno pro prostředí se 100 zaměstnanci a jedním IT administrátorem. Následující návrh představuje komplexní řešení s důrazem na českou lokalizaci, bezpečnost a efektivní správu.

## Architektura webové a vizualizační vrstvy

Architektura systému je navržena jako modulární s jasným oddělením odpovědností, což umožňuje snadnou údržbu a rozšiřitelnost:

```rust
// web/src/lib.rs
pub mod api;        // API rozhraní
pub mod components; // Znovupoužitelné komponenty
pub mod pages;      // Stránky aplikace
pub mod state;      // Správa stavu aplikace
pub mod utils;      // Pomocné funkce
pub mod visualization; // Vizualizační komponenty
```


### Backend implementace (Axum)

Pro backend využijeme framework Axum, který poskytuje vysoký výkon a bezpečnost:

```rust
// web/src/main.rs
use axum::{
    routing::{get, post},
    Router, Extension,
    http::{HeaderValue, Method},
    middleware::{self, Next},
    response::IntoResponse,
};
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

mod auth;
mod api;
mod db;
mod config;

#[tokio::main]
async fn main() -&gt; Result&lt;(), Box&lt;dyn std::error::Error&gt;&gt; {
    // Inicializace loggeru
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    // Načtení konfigurace
    let config = config::load_config()?;
    
    // Připojení k databázi
    let db_pool = db::create_connection_pool(&amp;config.database).await?;
    
    // Inicializace sdíleného stavu aplikace
    let app_state = Arc::new(AppState {
        db: db_pool,
        config: config.clone(),
        active_sessions: Mutex::new(HashMap::new()),
    });
    
    // Definice CORS pravidel
    let cors = CorsLayer::new()
        .allow_origin("https://siem.zoobrno.cz".parse::&lt;HeaderValue&gt;()?)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_credentials(true)
        .allow_headers(vec![http::header::AUTHORIZATION, http::header::CONTENT_TYPE]);
    
    // Definice API routeru
    let api_router = Router::new()
        .route("/auth/login", post(api::auth::login))
        .route("/auth/logout", post(api::auth::logout))
        .route("/auth/refresh", post(api::auth::refresh_token))
        // Tikety
        .route("/tickets", get(api::tickets::get_all_tickets))
        .route("/tickets", post(api::tickets::create_ticket))
        .route("/tickets/:id", get(api::tickets::get_ticket))
        .route("/tickets/:id", put(api::tickets::update_ticket))
        // Skripty
        .route("/scripts", get(api::scripts::get_all_scripts))
        .route("/scripts", post(api::scripts::create_script))
        .route("/scripts/:id", get(api::scripts::get_script))
        .route("/scripts/:id", put(api::scripts::update_script))
        .route("/scripts/:id/execute", post(api::scripts::execute_script))
        // Uživatelé
        .route("/users", get(api::users::get_all_users))
        .route("/users/:id", get(api::users::get_user))
        .route("/users/:id", put(api::users::update_user))
        // Dashboard
        .route("/dashboard/stats", get(api::dashboard::get_stats))
        .route("/dashboard/alerts", get(api::dashboard::get_alerts))
        // WebSocket pro real-time data
        .route("/ws/logs", get(api::ws::logs_handler))
        .route("/ws/alerts", get(api::ws::alerts_handler))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::jwt_auth_middleware
        ));
    
    // Hlavní router aplikace
    let app = Router::new()
        .nest("/api", api_router)
        .route("/health", get(|| async { "OK" }))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors)
        .with_state(app_state);
    
    // Spuštění serveru
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("Server běží na adrese {}", addr);
    
    axum::Server::bind(&amp;addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
```


### Frontend implementace (Leptos)

Pro frontend použijeme Leptos, moderní framework pro tvorbu reaktivních webových aplikací v Rustu:

```rust
// web/src/app.rs
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -&gt; impl IntoView {
    // Poskytovatel stavu aplikace
    provide_context(create_rw_signal(AppState::default()));
    
    view! {
        &lt;Stylesheet id="leptos" href="/pkg/zoo_siem.css"/&gt;
        &lt;Title text="SIEM ZOO Brno"/&gt;
        
        &lt;Router&gt;
            &lt;header&gt;
                &lt;NavBar /&gt;
            &lt;/header&gt;
            
            &lt;main&gt;
                &lt;Routes&gt;
                    &lt;Route path="/" view=Dashboard/&gt;
                    &lt;Route path="/tickets" view=TicketList/&gt;
                    &lt;Route path="/tickets/:id" view=TicketDetail/&gt;
                    &lt;Route path="/scripts" view=ScriptList/&gt;
                    &lt;Route path="/scripts/:id" view=ScriptDetail/&gt;
                    &lt;Route path="/users" view=UserList/&gt;
                    &lt;Route path="/settings" view=Settings/&gt;
                    &lt;Route path="/help" view=Help/&gt;
                    &lt;Route path="/*any" view=NotFound/&gt;
                &lt;/Routes&gt;
            &lt;/main&gt;
            
            &lt;footer&gt;
                &lt;Footer /&gt;
            &lt;/footer&gt;
        &lt;/Router&gt;
    }
}
```


## Implementace vizualizační vrstvy

Vizualizační vrstva je klíčovou součástí SIEM systému, protože umožňuje rychlé pochopení stavu sítě a bezpečnostních událostí.

### Síťový graf

```rust
// web/src/visualization/network_graph.rs
use leptos::*;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use plotly::{Plot, Scatter};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkNode {
    pub id: String,
    pub label: String,
    pub ip: String,
    pub status: String,
    pub type_: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkEdge {
    pub source: String,
    pub target: String,
    pub value: f64,
    pub label: String,
}

#[component]
pub fn NetworkGraph() -&gt; impl IntoView {
    let network_data = create_resource(
        || (), 
        |_| async move {
            // Získání dat ze serveru
            let resp = reqwest::get("/api/dashboard/network")
                .await
                .unwrap()
                .json::&lt;NetworkData&gt;()
                .await
                .unwrap();
            resp
        }
    );
    
    let canvas_ref = create_node_ref::&lt;HtmlCanvasElement&gt;();
    
    create_effect(move |_| {
        if let Some(data) = network_data.get() {
            if let Some(canvas) = canvas_ref.get() {
                // Inicializace grafu
                let mut plot = Plot::new();
                
                // Vytvoření uzlů
                let node_x: Vec&lt;f64&gt; = data.nodes.iter().map(|n| n.x).collect();
                let node_y: Vec&lt;f64&gt; = data.nodes.iter().map(|n| n.y).collect();
                let node_text: Vec&lt;String&gt; = data.nodes.iter().map(|n| format!("{} ({})", n.label, n.ip)).collect();
                
                let node_trace = Scatter::new(node_x, node_y)
                    .mode(plotly::common::Mode::Markers)
                    .marker(plotly::common::Marker::new().size(15))
                    .text_array(node_text)
                    .name("Zařízení");
                
                plot.add_trace(node_trace);
                
                // Vykreslení grafu
                plot.show();
            }
        }
    });
    
    view! {
        <div>
            <h2>"Síťový graf"</h2>
            <div>
                &lt;button on:click=|_| { /* Přiblížení */ }&gt;"Přiblížit"&lt;/button&gt;
                &lt;button on:click=|_| { /* Oddálení */ }&gt;"Oddálit"&lt;/button&gt;
                &lt;select&gt;
                    &lt;option&gt;"Všechna zařízení"&lt;/option&gt;
                    &lt;option&gt;"Pouze aktivní"&lt;/option&gt;
                    &lt;option&gt;"Pouze servery"&lt;/option&gt;
                &lt;/select&gt;
            </div>
            <div>
                &lt;canvas _ref=canvas_ref width="800" height="600"&gt;&lt;/canvas&gt;
            </div>
            <div>
                <div><span></span>"Server"</div>
                <div><span></span>"Pracovní stanice"</div>
                <div><span></span>"Síťové zařízení"</div>
            </div>
        </div>
    }
}
```


### Dashboard s grafy

```rust
// web/src/pages/dashboard.rs
use leptos::*;
use plotly::{Plot, Scatter, Layout};
use chrono::{DateTime, Utc, Duration};
use web_sys::HtmlDivElement;

#[component]
pub fn Dashboard() -&gt; impl IntoView {
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
    
    let alerts_resource = create_resource(
        || (), 
        |_| async move {
            let resp = reqwest::get("/api/dashboard/alerts")
                .await
                .unwrap()
                .json::&lt;Vec&lt;Alert&gt;&gt;()
                .await
                .unwrap();
            resp
        }
    );
    
    let traffic_plot_ref = create_node_ref::&lt;HtmlDivElement&gt;();
    
    // Vytvoření grafu síťového provozu
    create_effect(move |_| {
        if let Some(stats) = stats_resource.get() {
            if let Some(div) = traffic_plot_ref.get() {
                let mut plot = Plot::new();
                
                let time_points: Vec&lt;String&gt; = stats.traffic_history.iter()
                    .map(|p| p.timestamp.format("%H:%M").to_string())
                    .collect();
                
                let incoming: Vec&lt;f64&gt; = stats.traffic_history.iter()
                    .map(|p| p.incoming_mbps)
                    .collect();
                
                let outgoing: Vec&lt;f64&gt; = stats.traffic_history.iter()
                    .map(|p| p.outgoing_mbps)
                    .collect();
                
                let incoming_trace = Scatter::new(time_points.clone(), incoming)
                    .name("Příchozí provoz (Mbps)")
                    .line(plotly::common::Line::new().color("blue"));
                
                let outgoing_trace = Scatter::new(time_points, outgoing)
                    .name("Odchozí provoz (Mbps)")
                    .line(plotly::common::Line::new().color("green"));
                
                plot.add_trace(incoming_trace);
                plot.add_trace(outgoing_trace);
                
                let layout = Layout::new()
                    .title("Síťový provoz")
                    .height(300)
                    .margin(plotly::layout::Margin::new().top(30).bottom(30).left(50).right(20));
                
                plot.set_layout(layout);
                plot.show();
            }
        }
    });
    
    view! {
        <div>
            <h1>"SIEM Dashboard"</h1>
            
            <div>
                <div>
                    <h3>"Aktivní alerty"</h3>
                    <div>
                        {move || stats_resource.get().map(|s| s.active_alerts.to_string()).unwrap_or_else(|| "...".to_string())}
                    </div>
                </div>
                <div>
                    <h3>"Aktivní zařízení"</h3>
                    <div>
                        {move || stats_resource.get().map(|s| s.active_devices.to_string()).unwrap_or_else(|| "...".to_string())}
                    </div>
                </div>
                <div>
                    <h3>"Otevřené tikety"</h3>
                    <div>
                        {move || stats_resource.get().map(|s| s.open_tickets.to_string()).unwrap_or_else(|| "...".to_string())}
                    </div>
                </div>
                <div>
                    <h3>"Spuštěné skripty"</h3>
                    <div>
                        {move || stats_resource.get().map(|s| s.running_scripts.to_string()).unwrap_or_else(|| "...".to_string())}
                    </div>
                </div>
            </div>
            
            <div>
                <div>
                    <h2>"Síťový provoz"</h2>
                    <div></div>
                </div>
                
                <div>
                    <h2>"Poslední alerty"</h2>
                    <div>
                        {move || alerts_resource.get().map(|alerts| {
                            alerts.iter().take(5).map(|alert| {
                                view! {
                                    <div>
                                        <div>{alert.timestamp.format("%H:%M:%S").to_string()}</div>
                                        <div>{&amp;alert.message}</div>
                                        <div>{&amp;alert.source}</div>
                                    </div>
                                }
                            }).collect_view()
                        }).unwrap_or_else(|| view! { <div>"Načítání alertů..."</div> })}
                    </div>
                    <a href="/alerts">"Zobrazit všechny alerty"</a>
                </div>
            </div>
            
            <div>
                <div>
                    <h2>"Stav sítě"</h2>
                    &lt;NetworkGraph /&gt;
                </div>
                
                <div>
                    <h2>"Zdraví systému"</h2>
                    &lt;SystemHealthChart /&gt;
                </div>
            </div>
        </div>
    }
}
```


## Implementace tiketového systému

Tiketový systém je důležitou součástí SIEM řešení, protože umožňuje efektivní správu a řešení bezpečnostních incidentů.

```rust
// web/src/pages/tickets.rs
use leptos::*;
use leptos_router::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub created_at: String,
    pub created_by: String,
    pub assigned_to: Option&lt;String&gt;,
    pub category: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TicketStatus {
    New,
    InProgress,
    Waiting,
    Resolved,
    Closed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[component]
pub fn TicketList() -&gt; impl IntoView {
    let tickets = create_resource(
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
    
    let (filter, set_filter) = create_signal("all".to_string());
    
    let filtered_tickets = move || {
        tickets.get().map(|all_tickets| {
            match filter.get().as_str() {
                "new" =&gt; all_tickets.iter().filter(|t| t.status == TicketStatus::New).cloned().collect(),
                "inprogress" =&gt; all_tickets.iter().filter(|t| t.status == TicketStatus::InProgress).cloned().collect(),
                "waiting" =&gt; all_tickets.iter().filter(|t| t.status == TicketStatus::Waiting).cloned().collect(),
                "resolved" =&gt; all_tickets.iter().filter(|t| t.status == TicketStatus::Resolved).cloned().collect(),
                "closed" =&gt; all_tickets.iter().filter(|t| t.status == TicketStatus::Closed).cloned().collect(),
                "critical" =&gt; all_tickets.iter().filter(|t| t.priority == TicketPriority::Critical).cloned().collect(),
                _ =&gt; all_tickets,
            }
        }).unwrap_or_default()
    };
    
    view! {
        <div>
            <h1>"Správa tiketů"</h1>
            
            <div>
                <a href="/tickets/new">"Vytvořit nový tiket"</a>
                
                <div>
                    &lt;label for="ticket-filter"&gt;"Filtr:"&lt;/label&gt;
                    &lt;select id="ticket-filter" on:change=move |ev| {
                        let value = event_target_value(&amp;ev);
                        set_filter.set(value);
                    }&gt;
                        &lt;option value="all"&gt;"Všechny tikety"&lt;/option&gt;
                        &lt;option value="new"&gt;"Nové"&lt;/option&gt;
                        &lt;option value="inprogress"&gt;"V řešení"&lt;/option&gt;
                        &lt;option value="waiting"&gt;"Čekající"&lt;/option&gt;
                        &lt;option value="resolved"&gt;"Vyřešené"&lt;/option&gt;
                        &lt;option value="closed"&gt;"Uzavřené"&lt;/option&gt;
                        &lt;option value="critical"&gt;"Kritické"&lt;/option&gt;
                    &lt;/select&gt;
                </div>
            </div>
            
            <div>
                &lt;table&gt;
                    &lt;thead&gt;
                        &lt;tr&gt;
                            &lt;th&gt;"ID"&lt;/th&gt;
                            &lt;th&gt;"Název"&lt;/th&gt;
                            &lt;th&gt;"Stav"&lt;/th&gt;
                            &lt;th&gt;"Priorita"&lt;/th&gt;
                            &lt;th&gt;"Kategorie"&lt;/th&gt;
                            &lt;th&gt;"Vytvořeno"&lt;/th&gt;
                            &lt;th&gt;"Přiřazeno"&lt;/th&gt;
                            &lt;th&gt;"Akce"&lt;/th&gt;
                        &lt;/tr&gt;
                    &lt;/thead&gt;
                    &lt;tbody&gt;
                        &lt;Suspense fallback=move || view! { &lt;tr&gt;&lt;td colspan="8"&gt;"Načítání tiketů..."&lt;/td&gt;&lt;/tr&gt; }&gt;
                            {move || filtered_tickets().into_iter().map(|ticket| {
                                view! {
                                    &lt;tr class=format!("ticket-row priority-{:?}", ticket.priority).to_lowercase()&gt;
                                        &lt;td&gt;{&amp;ticket.id}&lt;/td&gt;
                                        &lt;td&gt;<a href='format!("/tickets/{}",'>{&amp;ticket.title}</a>&lt;/td&gt;
                                        &lt;td class=format!("status-{:?}", ticket.status).to_lowercase()&gt;
                                            {format!("{:?}", ticket.status)}
                                        &lt;/td&gt;
                                        &lt;td class=format!("priority-{:?}", ticket.priority).to_lowercase()&gt;
                                            {format!("{:?}", ticket.priority)}
                                        &lt;/td&gt;
                                        &lt;td&gt;{&amp;ticket.category}&lt;/td&gt;
                                        &lt;td&gt;{&amp;ticket.created_at}&lt;/td&gt;
                                        &lt;td&gt;{ticket.assigned_to.unwrap_or_else(|| "-".to_string())}&lt;/td&gt;
                                        &lt;td class="actions"&gt;
                                            <a href='format!("/tickets/{}",'>"Zobrazit"</a>
                                            <a href='format!("/tickets/{}/edit",'>"Upravit"</a>
                                        &lt;/td&gt;
                                    &lt;/tr&gt;
                                }
                            }).collect_view()}
                        &lt;/Suspense&gt;
                    &lt;/tbody&gt;
                &lt;/table&gt;
            </div>
        </div>
    }
}
```


## Správa skriptů

Správa PowerShell skriptů je klíčovou funkcionalitou pro administrátora ZOO Brno:

```rust
// web/src/pages/scripts.rs
use leptos::*;
use leptos_router::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Script {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content: String,
    pub created_at: String,
    pub created_by: String,
    pub last_run: Option&lt;String&gt;,
    pub run_count: u32,
    pub category: String,
    pub is_scheduled: bool,
    pub schedule: Option&lt;String&gt;,
    pub requires_approval: bool,
}

#[component]
pub fn ScriptDetail() -&gt; impl IntoView {
    let params = use_params_map();
    let script_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());
    
    let script = create_resource(
        script_id,
        |id| async move {
            let resp = reqwest::get(&amp;format!("/api/scripts/{}", id))
                .await
                .unwrap()
                .json::&lt;Script&gt;()
                .await
                .unwrap();
            resp
        }
    );
    
    let (is_running, set_is_running) = create_signal(false);
    let (output, set_output) = create_signal(String::new());
    
    let run_script = move |_| {
        let id = script_id();
        set_is_running.set(true);
        set_output.set("Spouštění skriptu...".to_string());
        
        spawn_local(async move {
            let resp = reqwest::post(&amp;format!("/api/scripts/{}/execute", id))
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            
            set_output.set(resp);
            set_is_running.set(false);
        });
    };
    
    view! {
        <div>
            &lt;Suspense fallback=move || view! { &lt;div&gt;"Načítání skriptu..."</div> }&gt;
                {move || script.get().map(|script| {
                    view! {
                        <div>
                            <h1>{&amp;script.name}</h1>
                            <div>
                                <span>{&amp;script.category}</span>
                                <span>"Vytvořil: " {&amp;script.created_by}</span>
                                <span>"Vytvořeno: " {&amp;script.created_at}</span>
                                <span>"Počet spuštění: " {script.run_count.to_string()}</span>
                            </div>
                            <p>{&amp;script.description}</p>
                        </div>
                        
                        <div>
                            <h2>"Kód skriptu"</h2>
                            <pre>{&amp;script.content}</pre>
                        </div>
                        
                        <div>
                            &lt;button 
                                class="button primary" 
                                on:click=run_script
                                disabled=is_running
                            &gt;
                                {if is_running() { "Skript běží..." } else { "Spustit skript" }}
                            &lt;/button&gt;
                            
                            <a href='format!("/scripts/{}/edit",'>
                                "Upravit skript"
                            </a>
                            
                            <a href='format!("/scripts/{}/history",'>
                                "Historie spuštění"
                            </a>
                            
                            &lt;button class="button" disabled=is_running&gt;
                                {if script.is_scheduled { "Zrušit plánování" } else { "Naplánovat" }}
                            &lt;/button&gt;
                        </div>
                        
                        <div>
                            <h2>"Výstup skriptu"</h2>
                            <pre>{output}</pre>
                        </div>
                    }
                }).unwrap_or_default()}
            &lt;/Suspense&gt;
        
    }
}
```


## Bezpečnostní opatření

Bezpečnost je klíčovým aspektem SIEM systému. Implementujeme následující bezpečnostní opatření:

```rust
// web/src/auth/jwt.rs
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub roles: Vec&lt;String&gt;,
}

pub async fn jwt_auth_middleware<b>(
    State(state): State&lt;Arc&lt;AppState&gt;&gt;,
    mut req: Request<b>,
    next: Next<b>,
) -&gt; Result&lt;Response, StatusCode&gt; {
    // Získání JWT tokenu z hlavičky
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|auth_value| {
            if auth_value.starts_with("Bearer ") {
                Some(auth_value[7..].to_owned())
            } else {
                None
            }
        });
    
    let token = match auth_header {
        Some(token) =&gt; token,
        None =&gt; return Err(StatusCode::UNAUTHORIZED),
    };
    
    // Dekódování a validace tokenu
    let token_data = match decode::&lt;Claims&gt;(
        &amp;token,
        &amp;DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &amp;Validation::new(Algorithm::HS256),
    ) {
        Ok(data) =&gt; data,
        Err(_) =&gt; return Err(StatusCode::UNAUTHORIZED),
    };
    
    // Kontrola expirace
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if token_data.claims.exp &lt; now {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // Přidání uživatelských informací do kontextu požadavku
    req.extensions_mut().insert(token_data.claims);
    
    // Pokračování k dalšímu middleware nebo handleru
    Ok(next.run(req).await)
}
```


## Integrace s Active Directory

Pro autentizaci a správu uživatelů implementujeme integraci s Active Directory:

```rust
// web/src/auth/ldap.rs
use ldap3::{LdapConn, Scope, SearchEntry};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LdapConfig {
    pub url: String,
    pub bind_dn: String,
    pub bind_password: String,
    pub search_base: String,
    pub user_filter: String,
    pub group_filter: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LdapUser {
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub groups: Vec&lt;String&gt;,
}

pub async fn authenticate_user(
    config: &amp;LdapConfig,
    username: &amp;str,
    password: &amp;str,
) -&gt; Result&lt;LdapUser, Box&lt;dyn Error&gt;&gt; {
    // Připojení k LDAP serveru
    let mut ldap = LdapConn::new(&amp;config.url)?;
    
    // Autentizace jako service account
    ldap.simple_bind(&amp;config.bind_dn, &amp;config.bind_password)?;
    
    // Vyhledání uživatele
    let user_filter = config.user_filter.replace("{username}", username);
    let search = ldap.search(
        &amp;config.search_base,
        Scope::Subtree,
        &amp;user_filter,
        vec!["cn", "mail", "displayName", "memberOf"],
    )?;
    
    let results = search.success()?;
    if results.0.len() != 1 {
        return Err("Uživatel nenalezen nebo nalezeno více uživatelů".into());
    }
    
    let entry = SearchEntry::construct(results.0[^0].clone());
    
    // Ověření hesla
    let user_dn = entry.dn.clone();
    let bind_result = ldap.simple_bind(&amp;user_dn, password);
    
    if bind_result.is_err() {
        return Err("Nesprávné heslo".into());
    }
    
    // Získání skupin
    let groups = entry.attrs.get("memberOf")
        .map(|g| g.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    
    // Vytvoření uživatelského objektu
    let user = LdapUser {
        username: username.to_string(),
        display_name: entry.attrs.get("displayName")
            .and_then(|v| v.first())
            .map(|s| s.to_string())
            .unwrap_or_else(|| username.to_string()),
        email: entry.attrs.get("mail")
            .and_then(|v| v.first())
            .map(|s| s.to_string())
            .unwrap_or_default(),
        groups,
    };
    
    Ok(user)
}
```


## Závěr a doporučení pro nasazení

Navržený SIEM systém pro ZOO Brno poskytuje komplexní řešení pro monitorování sítě, správu bezpečnostních incidentů a automatizaci běžných úkolů. Systém je optimalizován pro prostředí s jedním IT administrátorem a 100 zaměstnanci, s důrazem na bezpečnost, českou lokalizaci a uživatelskou přívětivost.

### Doporučení pro nasazení:

1. **Bezpečnostní testování**: Před nasazením do produkce proveďte penetrační testování systému pro odhalení případných zranitelností.
2. **Zálohování**: Implementujte pravidelné zálohování databáze a konfiguračních souborů.
3. **Monitorování výkonu**: Využijte Prometheus pro monitorování výkonu aplikace a včasnou detekci problémů.
4. **Školení uživatelů**: Proveďte školení zaměstnanců ZOO Brno pro efektivní využívání systému.
5. **Dokumentace**: Vytvořte podrobnou dokumentaci systému pro budoucí údržbu a rozšiřování.

Tento návrh poskytuje robustní základ pro SIEM systém ZOO Brno, který lze dále rozšiřovat podle specifických potřeb organizace.</b></b></b>

<div style="text-align: center">⁂</div>

[^1]: https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/64960960/38dd4496-e3bf-40b0-9c32-9f04e683abd6/paste.txt

[^2]: https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/collection_5f13ca3e-bb49-43f2-a6ab-0c0a14fa36f7/c26c131e-db6f-4d1e-b213-70096841e781/grok3-155min-CoT.txt

[^3]: https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/collection_5f13ca3e-bb49-43f2-a6ab-0c0a14fa36f7/35e73a79-dea4-45b7-97bd-a3d251faf07c/2022-03-07_Penetracni-testovani_v1.2.pdf

