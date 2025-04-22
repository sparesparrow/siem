<img src="https://r2cdn.perplexity.ai/pplx-full-logo-primary-dark%402x.png" class="logo" width="120"/>

# Implementace Grafany pro vizualizaci dat v SIEM systému ZOO Brno

Na základě poskytnutých informací je integrace Grafany jako hlavního nástroje pro vizualizaci dat v SIEM systému Zoo Brno velmi vhodným řešením. Tato změna přinese významné zlepšení uživatelského rozhraní a možnosti monitorování pro IT administrátora spravujícího síť se 100 zaměstnanci.

## Výhody použití Grafany v kontextu vašeho SIEM systému

1. **Nativní podpora PostgreSQL** - Grafana přímo podporuje vaši stávající PostgreSQL databázi[^1]
2. **Bohatá sada předpřipravených panelů** - Zahrnuje všechny potřebné typy vizualizací (statistiky, časové řady, síťové grafy, tabulky)[^1]
3. **Rust SDK pro Grafanu** - Existuje SDK pro vytváření backend pluginů v Rustu, což zajistí kompatibilitu s vaším současným kódem[^3][^5]
4. **Možnost automatizace** - API Grafany umožňuje programové vytváření a správu dashboardů pomocí Rustu[^1][^4]
5. **Široká komunita a dokumentace** - Poskytuje lepší dlouhodobou podporu než vlastní řešení vizualizací

## Implementační kroky

### 1. Integrace Grafany s existujícím backendem v Rustu

```rust
// backend/src/grafana/mod.rs

use reqwest::{Client, header};
use serde_json::{json, Value};
use std::error::Error;

pub struct GrafanaClient {
    api_url: String,
    api_key: String,
    client: Client,
}

impl GrafanaClient {
    pub fn new(api_url: String, api_key: String) -&gt; Self {
        let client = Client::new();
        Self { api_url, api_key, client }
    }
    
    /// Vytvoří nebo aktualizuje dashboard v Grafaně
    pub async fn import_dashboard(&amp;self, dashboard_json: Value) -&gt; Result&lt;String, Box&lt;dyn Error&gt;&gt; {
        let url = format!("{}/api/dashboards/db", self.api_url);
        
        let payload = json!({
            "dashboard": dashboard_json,
            "overwrite": true,
            "message": "Automaticky aktualizováno ze SIEM systému"
        });
        
        let response = self.client
            .post(&amp;url)
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&amp;payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Chyba při importu dashboardu: {}", error_text).into());
        }
        
        let response_data: Value = response.json().await?;
        Ok(response_data["url"].as_str().unwrap_or("").to_string())
    }
    
    /// Získá existující dashboard podle UID
    pub async fn get_dashboard(&amp;self, uid: &amp;str) -&gt; Result&lt;Value, Box&lt;dyn Error&gt;&gt; {
        let url = format!("{}/api/dashboards/uid/{}", self.api_url, uid);
        
        let response = self.client
            .get(&amp;url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Chyba při získávání dashboardu: {}", error_text).into());
        }
        
        let dashboard: Value = response.json().await?;
        Ok(dashboard)
    }
}
```


### 2. Konfigurace a inicializace v hlavní aplikaci

```rust
// backend/src/config/mod.rs

#[derive(Debug, Deserialize, Clone)]
pub struct GrafanaConfig {
    pub url: String,
    pub api_key: String,
    pub datasource_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    // Existující konfigurace
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    // Přidáme sekci pro Grafanu
    pub grafana: GrafanaConfig,
}
```

```rust
// backend/src/main.rs
use crate::grafana::GrafanaClient;

#[tokio::main]
async fn main() -&gt; Result&lt;(), Box&lt;dyn std::error::Error&gt;&gt; {
    // Načtení konfigurace
    let config = config::load_config()?;
    
    // Inicializace Grafana klienta
    let grafana_client = GrafanaClient::new(
        config.grafana.url.clone(),
        config.grafana.api_key.clone()
    );
    
    // Registrace vizualizačních endpointů
    let app = Router::new()
        // ... existující routy
        .nest("/api/visualization", visualization_router(grafana_client.clone()))
        .with_state(AppState {
            // Existující stav
            grafana: grafana_client,
        });
        
    // ... zbytek aplikace
}
```


### 3. Vytvoření API endpointů pro správu dashboardů

```rust
// backend/src/api/visualization.rs

use axum::{
    Router,
    routing::{get, post},
    extract::{State, Path, Json},
    http::StatusCode,
};
use serde_json::Value;
use crate::grafana::GrafanaClient;

// Templates dashboardů pro různé účely
pub async fn get_dashboard_templates() -&gt; Json&lt;Vec&lt;DashboardTemplate&gt;&gt; {
    Json(vec![
        DashboardTemplate {
            id: "network_overview",
            name: "Přehled sítě",
            description: "Kompletní monitoring síťového provozu a stavu zařízení",
        },
        DashboardTemplate {
            id: "security_dashboard",
            name: "Bezpečnostní přehled",
            description: "Monitoring bezpečnostních událostí a alertů",
        },
        // Další šablony
    ])
}

// Endpoint pro vytvoření dashboardu ze šablony
pub async fn create_dashboard_from_template(
    State(state): State&lt;AppState&gt;,
    Path(template_id): Path&lt;String&gt;,
) -&gt; Result&lt;Json&lt;Value&gt;, StatusCode&gt; {
    let template = match template_id.as_str() {
        "network_overview" =&gt; include_str!("../dashboards/network_overview.json"),
        "security_dashboard" =&gt; include_str!("../dashboards/security_dashboard.json"),
        // Další šablony
        _ =&gt; return Err(StatusCode::NOT_FOUND),
    };
    
    let dashboard_json: Value = serde_json::from_str(template)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let url = state.grafana.import_dashboard(dashboard_json)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(json!({ "url": url })))
}

pub fn visualization_router(grafana: GrafanaClient) -&gt; Router {
    Router::new()
        .route("/templates", get(get_dashboard_templates))
        .route("/templates/:id/create", post(create_dashboard_from_template))
        .with_state(grafana)
}
```


### 4. Vytvoření Grafana pluginu v Rustu pomocí SDK

Pro pokročilejší integraci můžete vytvořit vlastní backend plugin pro Grafanu s využitím Rust SDK[^3][^5]:

```rust
// grafana-plugin/src/main.rs
use grafana_plugin_sdk::{backend, data};
use anyhow::Result;

// Definice handleru pro dotazy z Grafany
pub struct ZooSiemHandler {}

#[async_trait::async_trait]
impl backend::QueryDataHandler for ZooSiemHandler {
    async fn query_data(&amp;self, req: backend::QueryDataRequest) -&gt; Result&lt;backend::QueryDataResponse&gt; {
        let mut response = backend::QueryDataResponse::new();

        for query in req.queries {
            // Vytvoření datasetu na základě dotazu z Grafany
            let frame = match query.query_type.as_str() {
                "network_status" =&gt; query_network_status(&amp;query).await?,
                "active_alerts" =&gt; query_active_alerts(&amp;query).await?,
                "ticket_distribution" =&gt; query_ticket_distribution(&amp;query).await?,
                // Další typy dotazů
                _ =&gt; data::Frame::new("default").unwrap(),
            };

            response.responses.insert(query.ref_id, backend::DataResponse::new(frame));
        }

        Ok(response)
    }
}

// Implementace jednotlivých typů dotazů
async fn query_network_status(query: &amp;backend::DataQuery) -&gt; Result&lt;data::Frame&gt; {
    // Implementace dotazu na stav sítě z vaší PostgreSQL databáze
    // ...
}

#[tokio::main]
async fn main() -&gt; Result&lt;()&gt; {
    // Registrace handleru pro dotazy
    let mut handler = backend::PluginBackend::new();
    handler.register_query_handler(ZooSiemHandler {});
    
    // Spuštění plugin serveru
    backend::serve(handler).await?;
    Ok(())
}
```


### 5. Přidání předdefinovaných dashboardů

Součástí implementace by měly být předdefinované dashboardy pro klíčové oblasti monitoringu:

```json
// backend/src/dashboards/network_overview.json
{
  "dashboard": {
    "id": null,
    "uid": "zoo-network-overview",
    "title": "Přehled sítě ZOO Brno",
    "description": "Monitoring síťového provozu a stavu zařízení v ZOO Brno",
    "tags": ["network", "monitoring", "zoo"],
    "timezone": "browser",
    "schemaVersion": 36,
    "version": 1,
    "refresh": "5s",
    "panels": [
      {
        "id": 1,
        "title": "Aktivní zařízení",
        "type": "stat",
        "datasource": "${DS_POSTGRESQL}",
        "targets": [
          {
            "refId": "A",
            "rawSql": "SELECT COUNT(*) FROM devices WHERE status = 'online'"
          }
        ],
        "options": {
          "colorMode": "value",
          "graphMode": "area",
          "justifyMode": "auto",
          "textMode": "auto"
        },
        "fieldConfig": {
          "defaults": {
            "color": {
              "mode": "thresholds"
            },
            "thresholds": {
              "mode": "absolute",
              "steps": [
                { "color": "red", "value": null },
                { "color": "orange", "value": 50 },
                { "color": "green", "value": 80 }
              ]
            }
          }
        },
        "gridPos": { "h": 8, "w": 6, "x": 0, "y": 0 }
      },
      // Další panely...
    ]
  }
}
```


## Integrace s existující architekturou

### Použití Leptos a Grafany

Pro integraci Grafany do vaší Leptos frontend aplikace doporučuji:

```rust
// web/src/components/grafana_iframe.rs
use leptos::*;

#[component]
pub fn GrafanaDashboard(
    #[prop(default = "zoo-network-overview")] dashboard_uid: &amp;'static str,
    #[prop(default = 8)] height_in_rem: u32
) -&gt; impl IntoView {
    let grafana_url = use_context::&lt;Signal&lt;String&gt;&gt;()
        .expect("GrafanaURL nebyl nastaven v kontextu");
    
    let dashboard_url = move || {
        format!(
            "{}/d/{}?orgId=1&amp;theme=light",
            grafana_url.get(),
            dashboard_uid
        )
    };
    
    view! {
        <div>
            &lt;iframe 
                src={dashboard_url()}
                width="100%" 
                height={format!("{}rem", height_in_rem)}
                frameborder="0"
            &gt;&lt;/iframe&gt;
        </div>
    }
}
```

Tuto komponentu můžete následně použít v různých částech vašeho UI:

```rust
// web/src/pages/dashboard.rs
use leptos::*;
use crate::components::grafana_iframe::GrafanaDashboard;

#[component]
pub fn Dashboard() -&gt; impl IntoView {
    view! {
        <div>
            <h1>"SIEM Dashboard"</h1>
            
            <div>
                &lt;GrafanaDashboard 
                    dashboard_uid="zoo-overview-summary" 
                    height_in_rem=10
                /&gt;
            </div>
            
            <div>
                <div>
                    <h2>"Stav sítě"</h2>
                    &lt;GrafanaDashboard 
                        dashboard_uid="zoo-network-status" 
                        height_in_rem=20
                    /&gt;
                </div>
                
                <div>
                    <h2>"Bezpečnostní alerty"</h2>
                    &lt;GrafanaDashboard 
                        dashboard_uid="zoo-security-alerts" 
                        height_in_rem=20
                    /&gt;
                </div>
            </div>
        </div>
    }
}
```


## Bezpečnostní aspekty integrace

Při integraci Grafany je třeba vzít v úvahu bezpečnostní aspekty zmíněné v dokumentu NÚKIB[^2]:

1. **Zabezpečený přístup**:
    - API klíče Grafany ukládejte do bezpečného úložiště, nikoliv přímo do zdrojového kódu
    - Implementujte JWT autentizaci mezi vaší aplikací a Grafanou
    - Omezte přístup k API Grafany pouze na nezbytné operace
2. **Auditní logování**:
    - Zaznamenávejte veškeré operace prováděné prostřednictvím Grafana API
    - Logujte přístupy k dashboardům pro forenzní účely
3. **Oddělení prostředí**:
    - Zvažte oddělení vývojové, testovací a produkční instance Grafany
    - Implementujte CI/CD pipeline pro bezpečné nasazení dashboardů
4. **Penetrační testování**:
    - Zahrňte Grafanu do rozsahu penetračního testování
    - Testujte CORS nastavení a další bezpečnostní prvky

## Implementace požadovaných vizualizací

### Příklady SQL dotazů pro panely v Grafaně

| Sekce | Typ panelu | SQL dotaz |
| :-- | :-- | :-- |
| Dashboard přehled | Stat | `SELECT COUNT(*) FROM alerts WHERE status = 'active'` |
| Síťový provoz | Time Series | `SELECT time, incoming_mbps, outgoing_mbps FROM network_traffic WHERE time &gt;= now() - interval '24 hours'` |
| Distribuce tiketů | Pie | `SELECT status, COUNT(*) AS count FROM tickets GROUP BY status` |
| Síťová zařízení | Node Graph | `WITH nodes AS (SELECT id, name AS title, status AS mainStat FROM devices), edges AS (SELECT source_id AS source, target_id AS target, bandwidth AS mainStat FROM connections) SELECT * FROM nodes UNION ALL SELECT * FROM edges` |
| Tiskárny | Bar Gauge | `SELECT name, toner_level FROM printers ORDER BY toner_level` |
| Skriptová aktivita | Table | `SELECT name, last_run, status, duration FROM scripts ORDER BY last_run DESC LIMIT 10` |

## Závěr a doporučení

Implementace Grafany do SIEM systému ZOO Brno představuje optimální řešení pro vizualizační část projektu z následujících důvodů:

1. **Efektivita vývoje** - Využití již existujícího, otestovaného nástroje místo vytváření vlastních vizualizací od základu
2. **Jednoduchá údržba** - Aktualizace a správa vizualizací je značně jednodušší přes Grafana UI než úpravami kódu
3. **Rozšiřitelnost** - Snadné přidávání nových dashboardů a vizualizací podle potřeby
4. **Integrace s Rustem** - Existující SDK a API umožňují bezproblémovou integraci s vaším současným backendem
5. **Automatizace** - Možnost programového vytváření a správy dashboardů usnadní práci IT administrátorovi

### Další kroky:

1. Nastavte instanci Grafany a připojte ji k vaší PostgreSQL databázi
2. Implementujte základní dashboardy pro klíčové oblasti monitoringu
3. Integrujte Grafana iframe komponenty do vašeho Leptos frontendu
4. Pokud potřebujete pokročilejší integraci, implementujte vlastní backend plugin pomocí Rust SDK
5. Zahrňte Grafanu do penetračního testování vašeho SIEM systému

Tento přístup zajistí efektivní, moderní a bezpečné řešení vizualizací pro váš SIEM systém s minimálním úsilím při implementaci a údržbě.

<div style="text-align: center">⁂</div>

[^1]: https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/64960960/ead781a6-ba88-4e90-9ed3-d5efe3ff2a6c/paste.txt

[^2]: https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/collection_5f13ca3e-bb49-43f2-a6ab-0c0a14fa36f7/35e73a79-dea4-45b7-97bd-a3d251faf07c/2022-03-07_Penetracni-testovani_v1.2.pdf

[^3]: https://github.com/grafana/grafana-plugin-sdk-rust

[^4]: https://codezup.com/building-a-real-time-analytics-dashboard-with-rust-and-grafana/

[^5]: https://github.com/sd2k/grafana-sample-backend-plugin-rust

[^6]: https://grafana.com/docs/beyla/latest/quickstart/rust/

[^7]: https://grafana.com/docs/grafana/latest/dashboards/build-dashboards/import-dashboards/

[^8]: https://www.shuttle.dev/blog/2024/03/28/grafana-rust

[^9]: https://techblog.commercetools.com/adding-consistency-and-automation-to-grafana-e99eb374fe40

[^10]: https://logit.io/blog/post/top-grafana-dashboards-and-visualisations/

[^11]: https://dev.to/nithya15aa/automating-dashboard-import-for-grafana-standalone-setup-using-provisioning-59ga

[^12]: https://grafana.com/grafana/dashboards/17574-rust-server-metrics/

[^13]: https://grafana.com/grafana/dashboards/20840-rust-server-metrics/

[^14]: https://edgedelta.com/company/blog/grafana-pros-and-cons

[^15]: https://www.develeap.com/Grafana-The-API-Magic/

[^16]: https://crates.io/crates/grafana-plugin-sdk

[^17]: https://www.reddit.com/r/rust/comments/qqol3n/monitoring_rust_web_application_with_prometheus/

[^18]: https://grafana.com/grafana/dashboards/18823-akamai-siem/

[^19]: https://community.grafana.com/t/how-to-import-dashboard-in-grafana-using-rest-api/73182

[^20]: https://grafana.com/docs/pyroscope/latest/configure-client/language-sdks/rust/

[^21]: https://www.reddit.com/r/rust/comments/rbvmib/announcing_the_grafana_plugin_sdk_for_rust/

[^22]: https://www.hivemq.com/blog/mqtt-data-visualization-with-grafana/

