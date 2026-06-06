use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

#[derive(Serialize, Clone)]
struct LogEntry {
    timestamp: String,
    level: String,
    message: String,
}

#[derive(Serialize)]
struct StatusResponse {
    wallet: String,
    rpc: String,
    running: bool,
    total_bundles: usize,
}

#[derive(Deserialize)]
struct RunConfig {
    congestion: Option<String>,
    runs: Option<u32>,
}

struct AppState {
    logs: Arc<Mutex<Vec<LogEntry>>>,
    running: Arc<Mutex<bool>>,
}

#[get("/api/status")]
async fn status(data: web::Data<AppState>) -> impl Responder {
    let running = *data.running.lock().unwrap();
    let logs = data.logs.lock().unwrap();
    HttpResponse::Ok().json(StatusResponse {
        wallet: "BKJpv6cGtaaH7fNMogZavxfczSXXb75cWBiDTWqGvDPz".to_string(),
        rpc: std::env::var("SOLANA_RPC").unwrap_or_default(),
        running,
        total_bundles: logs.len(),
    })
}

#[get("/api/logs")]
async fn get_logs(data: web::Data<AppState>) -> impl Responder {
    let logs = data.logs.lock().unwrap();
    HttpResponse::Ok().json(logs.clone())
}

#[get("/api/lifecycle")]
async fn get_lifecycle() -> impl Responder {
    match std::fs::read_to_string("logs/lifecycle.jsonl") {
        Ok(content) => {
            let entries: Vec<serde_json::Value> = content
                .lines()
                .filter(|l| !l.is_empty())
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();
            HttpResponse::Ok().json(entries)
        }
        Err(_) => HttpResponse::Ok().json(Vec::<serde_json::Value>::new()),
    }
}

#[post("/api/run")]
async fn run_bundle(
    data: web::Data<AppState>,
    config: web::Json<RunConfig>,
) -> impl Responder {
    let running = {
        let r = data.running.lock().unwrap();
        *r
    };

    if running {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": "BundleIQ is already running"
        }));
    }

    *data.running.lock().unwrap() = true;
    data.logs.lock().unwrap().clear();

    let logs = data.logs.clone();
    let running_flag = data.running.clone();
    let runs = config.runs.unwrap_or(1);

    actix_web::rt::spawn(async move {
        for _ in 0..runs {
            let output = Command::new("./target/debug/bundleiq")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();

            match output {
                Ok(mut child) => {
                    if let Some(stderr) = child.stderr.take() {
                        let reader = BufReader::new(stderr);
                        for line in reader.lines().flatten() {
                            let entry = parse_log_line(&line);
                            logs.lock().unwrap().push(entry);
                        }
                    }
                    let _ = child.wait();
                }
                Err(e) => {
                    logs.lock().unwrap().push(LogEntry {
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        level: "ERROR".to_string(),
                        message: format!("Failed to start bundleiq: {}", e),
                    });
                }
            }
        }
        *running_flag.lock().unwrap() = false;
    });

    HttpResponse::Ok().json(serde_json::json!({
        "status": "started",
        "runs": runs
    }))
}

#[get("/api/stream")]
async fn stream_logs(data: web::Data<AppState>) -> impl Responder {
    let logs = data.logs.lock().unwrap().clone();
    let body = logs
        .iter()
        .map(|l| format!("data: {}\n\n", serde_json::to_string(l).unwrap()))
        .collect::<Vec<_>>()
        .join("");

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .append_header(("Cache-Control", "no-cache"))
        .body(body)
}

fn parse_log_line(line: &str) -> LogEntry {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let level = if line.contains("ERROR") {
        "ERROR"
    } else if line.contains("WARN") {
        "WARN"
    } else {
        "INFO"
    }.to_string();

    let message = if let Some(idx) = line.find("bundleiq") {
        line[idx..].to_string()
    } else {
        line.to_string()
    };

    LogEntry { timestamp, level, message }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let port: u16 = port.parse().unwrap_or(8080);

    println!("[Server] BundleIQ API starting on port {}", port);

    let state = web::Data::new(AppState {
        logs: Arc::new(Mutex::new(Vec::new())),
        running: Arc::new(Mutex::new(false)),
    });

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(state.clone())
            .service(status)
            .service(get_logs)
            .service(get_lifecycle)
            .service(run_bundle)
            .service(stream_logs)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
