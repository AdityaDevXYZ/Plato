use std::env;
use std::fs;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const DEFAULT_API_HOST: &str = "127.0.0.1";
const DEFAULT_API_PORT: u16 = 8080;

fn token_path() -> std::path::PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home).join(".plato_token")
}

fn get_stored_token() -> Option<String> {
    if let Ok(tok) = env::var("PLATO_TOKEN") {
        if !tok.trim().is_empty() {
            return Some(tok.trim().to_string());
        }
    }
    fs::read_to_string(token_path()).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

async fn http_request(method: &str, path: &str, host: &str, port: u16, body_opt: Option<&str>, token_opt: Option<&str>) -> anyhow::Result<(u16, String)> {
    let addr = format!("{}:{}", host, port);
    let mut stream = tokio::time::timeout(Duration::from_secs(4), TcpStream::connect(&addr))
        .await
        .map_err(|_| anyhow::anyhow!("Connection timeout to PLATO API at {}", addr))??;

    let auth_header = match token_opt {
        Some(token) if !token.is_empty() => format!("Authorization: Bearer {}\r\n", token),
        _ => String::new(),
    };

    let (content_type_header, content_length_header, body_str) = match body_opt {
        Some(b) => (
            "Content-Type: application/json\r\n",
            format!("Content-Length: {}\r\n", b.len()),
            b,
        ),
        None => ("", String::new(), ""),
    };

    let req = format!(
        "{} {} HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: plato-cli/1.0\r\n{}{}{}Connection: close\r\n\r\n{}",
        method, path, host, port, auth_header, content_type_header, content_length_header, body_str
    );
    stream.write_all(req.as_bytes()).await?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    let resp = String::from_utf8_lossy(&buf);

    let parts: Vec<&str> = resp.splitn(2, "\r\n\r\n").collect();
    let header = parts.get(0).copied().unwrap_or("");
    let body = parts.get(1).copied().unwrap_or("").to_string();

    let status_code: u16 = header
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or(500);

    Ok((status_code, body))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let host = env::var("PLATO_API_HOST").unwrap_or_else(|_| DEFAULT_API_HOST.to_string());
    let port: u16 = env::var("PLATO_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_API_PORT);

    let token = get_stored_token();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match command {
        "status" => {
            println!("🛰️  ================================================================");
            println!("🛰️  PLATO: Mission Control DevSecOps CLI (v1.0.0)");
            println!("🛰️  ================================================================");
            println!("Target API: http://{}:{}", host, port);

            match http_request("GET", "/health", &host, port, None, token.as_deref()).await {
                Ok((200, body)) => {
                    println!("[✓] PLATO API Gateway is ONLINE");
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        println!("    Status:  {}", json.get("status").and_then(|v| v.as_str()).unwrap_or("healthy"));
                    }
                    if let Some(tok) = &token {
                        println!("    Operator Session: Authenticated ({})", &tok[..tok.len().min(12)]);
                    } else {
                        println!("    Operator Session: Unauthenticated (Run `plato-cli login` to authenticate)");
                    }
                }
                Ok((code, body)) => println!("[!] API returned HTTP {}: {}", code, body),
                Err(e) => {
                    println!("[x] API Offline or unreachable: {}", e);
                    println!("    Ensure PLATO is running via `start.sh` or `docker compose up -d`.");
                }
            }
        }
        "login" => {
            let email = args.get(2).map(|s| s.as_str()).unwrap_or("admin@plato.aero");
            let password = args.get(3).map(|s| s.as_str()).unwrap_or("PlatoAdmin2026!");

            println!("Authenticating operator '{}' against http://{}:{}...", email, host, port);
            let payload = serde_json::json!({ "email": email, "password": password }).to_string();

            match http_request("POST", "/api/v1/auth/login", &host, port, Some(&payload), None).await {
                Ok((200, body)) => {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(jwt) = json.get("token").and_then(|v| v.as_str()) {
                            let role = json.get("role").and_then(|v| v.as_str()).unwrap_or("Operator");
                            let _ = fs::write(token_path(), jwt);
                            println!("[✓] Authentication Successful!");
                            println!("    Role:     {}", role);
                            println!("    Session:  Saved to {:?}", token_path());
                            return Ok(());
                        }
                    }
                    println!("[!] Unexpected response: {}", body);
                }
                Ok((code, body)) => println!("[x] Login Failed (HTTP {}): {}", code, body),
                Err(e) => println!("[x] Connection error: {}", e),
            }
        }
        "health" => {
            println!("Running PLATO Health Checks against http://{}:{}...", host, port);
            for (name, path) in [
                ("System Health", "/health"),
                ("Liveness Probe", "/health/live"),
                ("Readiness Probe", "/health/ready"),
            ] {
                match http_request("GET", path, &host, port, None, token.as_deref()).await {
                    Ok((200, _)) => println!("  ✓ {} ({}): 200 OK", name, path),
                    Ok((code, body)) => println!("  ! {} ({}): HTTP {} - {}", name, path, code, body),
                    Err(e) => println!("  x {} ({}): Error - {}", name, path, e),
                }
            }
        }
        "satellites" | "twin" => {
            println!("Querying Constellation Digital Twin from http://{}:{}...", host, port);
            match http_request("GET", "/api/v1/twin/satellites", &host, port, None, token.as_deref()).await {
                Ok((200, body)) => {
                    if let Ok(sats) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
                        println!("\nActive Constellation Assets ({} total):", sats.len());
                        println!("{:<14} {:<12} {:<10} {:<8} {:<10} {:<10}", "ID", "Name", "Status", "Health", "Battery", "Temp");
                        println!("{:-<68}", "");
                        for s in sats {
                            let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                            let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed");
                            let status = s.get("operational_status").and_then(|v| v.as_str()).unwrap_or("Active");
                            let health = s.get("health_score").and_then(|v| v.as_i64()).unwrap_or(100);
                            let bat = s.get("battery_level").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let temp = s.get("temperature").and_then(|v| v.as_f64()).unwrap_or(20.0);
                            println!("{:<14} {:<12} {:<10} {:<8}% {:<9.1}% {:<9.1}°C", &id[..id.len().min(12)], name, status, health, bat, temp);
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Ok((code, body)) => println!("[!] HTTP {}: {}", code, body),
                Err(e) => println!("[x] Error querying digital twin: {}", e),
            }
        }
        "deployments" | "queue" => {
            println!("Querying Deployment Execution Queue from http://{}:{}...", host, port);
            match http_request("GET", "/api/v1/deployments/queue", &host, port, None, token.as_deref()).await {
                Ok((200, body)) => {
                    if let Ok(queue) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
                        println!("\nActive & Queued Deployment Sessions ({} total):", queue.len());
                        println!("{:<14} {:<18} {:<12} {:<10}", "Session ID", "Mission / Release", "Status", "Progress");
                        println!("{:-<60}", "");
                        for s in queue {
                            let sid = s.get("id").or_else(|| s.get("session_id")).and_then(|v| v.as_str()).unwrap_or("?");
                            let mission = s.get("mission").and_then(|v| v.as_str()).unwrap_or("Release");
                            let status = s.get("status").and_then(|v| v.as_str()).unwrap_or("Queued");
                            let prog = s.get("progress").and_then(|v| v.as_i64()).unwrap_or(0);
                            println!("{:<14} {:<18} {:<12} {}%", &sid[..sid.len().min(12)], mission, status, prog);
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Ok((401, _)) => {
                    println!("[!] HTTP 401 Unauthorized: Please run `plato-cli login` first.");
                }
                Ok((code, body)) => println!("[!] HTTP {}: {}", code, body),
                Err(e) => println!("[x] Error querying deployments: {}", e),
            }
        }
        "audit-verify" => {
            println!("Verifying Cryptographic SHA-256 Audit Ledger Chain...");
            match http_request("GET", "/api/v1/audit/verify", &host, port, None, token.as_deref()).await {
                Ok((200, body)) => {
                    println!("[✓] Audit Chain Verified: {}", body);
                }
                Ok((401, _)) => {
                    println!("[!] HTTP 401 Unauthorized: Please run `plato-cli login` first.");
                }
                Ok((code, body)) => println!("[!] HTTP {}: {}", code, body),
                Err(e) => println!("[x] Error verifying audit chain: {}", e),
            }
        }
        "orbits" => {
            println!("Querying Astrodynamics Ephemeris & Ground Stations from http://{}:{}...", host, port);
            if let Ok((200, body)) = http_request("GET", "/api/v1/orbits/stations", &host, port, None, token.as_deref()).await {
                if let Ok(stations) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
                    println!("\nActive Ground Stations ({} total):", stations.len());
                    println!("{:<36} {:<10} {:<10} {:<8}", "Station Name", "Latitude", "Longitude", "Alt(m)");
                    println!("{:-<68}", "");
                    for gs in stations {
                        let name = gs.get("name").and_then(|v| v.as_str()).unwrap_or("Station");
                        let lat = gs.get("latitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let lon = gs.get("longitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let alt = gs.get("altitude_m").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        println!("{:<36} {:<10.4} {:<10.4} {:<8.1}", name, lat, lon, alt);
                    }
                }
            }

            println!("\nConstellation Orbital Ephemeris:");
            for (sat_num, sid) in [
                (1, "aaaaaaaa-0001-0000-0000-000000000001"),
                (2, "aaaaaaaa-0002-0000-0000-000000000002"),
                (3, "aaaaaaaa-0003-0000-0000-000000000003"),
            ] {
                if let Ok((200, body)) = http_request("GET", &format!("/api/v1/orbits/{}", sid), &host, port, None, token.as_deref()).await {
                    if let Ok(orb) = serde_json::from_str::<serde_json::Value>(&body) {
                        let geo = orb.get("geodetic");
                        let lat = geo.and_then(|g| g.get("latitude")).and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let lon = geo.and_then(|g| g.get("longitude")).and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let alt = geo.and_then(|g| g.get("altitude_km")).and_then(|v| v.as_f64()).unwrap_or(0.0);
                        println!("  🛰️  Alpha-{} SGP4 Ephemeris: Lat={:.2}°, Lon={:.2}°, Alt={:.2} km", sat_num, lat, lon, alt);
                    }
                }
            }
        }
        "release-cycle" => {
            let tok = match &token {
                Some(t) => t.as_str(),
                None => {
                    println!("[x] Error: Operator must authenticate first. Run `plato-cli login`.");
                    return Ok(());
                }
            };
            let version = args.get(2).map(|s| s.as_str()).unwrap_or("v1.3.0");
            let mission_id = "11111111-1111-1111-1111-111111111111";

            println!("🚀 ================================================================");
            println!("🚀 Executing PLATO Full Release Lifecycle (ROE -> RAE -> DRA -> DEE)");
            println!("🚀 Target Version: {} | Mission: {}", version, mission_id);
            println!("🚀 ================================================================");

            // Step 1: Create Release
            println!("\n[1/7] Creating release draft in Release Orchestration Engine (ROE)...");
            let req_body = serde_json::json!({
                "version": version,
                "mission_id": mission_id
            }).to_string();

            let (code, body) = http_request("POST", "/api/v1/releases", &host, port, Some(&req_body), Some(tok)).await?;
            if code != 200 {
                println!("[x] Failed to create release (HTTP {}): {}", code, body);
                return Ok(());
            }
            let rel_json: serde_json::Value = serde_json::from_str(&body)?;
            let rel_id = rel_json.get("id").and_then(|v| v.as_str()).unwrap();
            let release_uuid = uuid::Uuid::parse_str(rel_id)?;
            println!("      ✓ Release ID: {} (Status: Draft)", rel_id);

            // Step 2: Register Manifest with Ed25519 Cryptographic Signatures
            println!("\n[2/7] Generating Ed25519 signed flight binary release manifest...");
            let seed = [42u8; 32];
            let dummy_firmware = b"PLATO_FLIGHT_FIRMWARE_PAYLOAD_V1.3.0_CORTEX_R5";
            let sha256_hash = assurance::checks::compute_sha256(dummy_firmware);

            let mut manifest = assurance::model::FlightReleaseManifest {
                release_id: types::ReleaseId::from(release_uuid),
                version: version.to_string(),
                target_architecture: "ARM-Cortex-R5".to_string(),
                max_binary_size_bytes: 33554432,
                dual_bank_enabled: true,
                artifacts: vec![assurance::model::BinaryArtifact {
                    artifact_id: uuid::Uuid::new_v4(),
                    name: "flight_core.bin".to_string(),
                    target_partition: "bank_b".to_string(),
                    size_bytes: dummy_firmware.len() as u64,
                    expected_sha256: sha256_hash.clone(),
                    payload_hex: Some(hex::encode(dummy_firmware)),
                }],
                signatures: Vec::new(),
                created_at: time::OffsetDateTime::now_utc(),
            };

            let digest = manifest.compute_canonical_digest();
            let (pubkey_hex, sig_hex) = assurance::checks::sign_ed25519_message(&seed, &digest);
            manifest.signatures.push(assurance::model::CryptographicSignature {
                signer_id: "sec-ops-key-01".to_string(),
                algorithm: "Ed25519".to_string(),
                public_key_hex: pubkey_hex,
                signature_hex: sig_hex,
            });

            let man_body = serde_json::to_string(&manifest)?;
            let (code, body) = http_request("POST", &format!("/api/v1/releases/{}/manifest", rel_id), &host, port, Some(&man_body), Some(tok)).await?;
            if code != 200 {
                println!("[x] Failed to register manifest (HTTP {}): {}", code, body);
                return Ok(());
            }
            println!("      ✓ Manifest attached with ARM-Cortex-R5 target, Dual-Bank, SHA-256 and Ed25519 sign-off");

            // Step 3: Validate Release
            println!("\n[3/8] Validating release prerequisites...");
            let val_body = serde_json::json!({ "reason": "Manifest and integrity verified" }).to_string();
            let (code, body) = http_request("POST", &format!("/api/v1/releases/{}/validate", rel_id), &host, port, Some(&val_body), Some(tok)).await?;
            if code != 200 {
                println!("[x] Validation failed (HTTP {}): {}", code, body);
                return Ok(());
            }
            println!("      ✓ Release state transitioned to Validated");

            // Step 4: Submit Release
            println!("\n[4/8] Submitting release for formal mission control review...");
            let sub_body = serde_json::json!({ "reason": "Pre-flight engineering submission complete" }).to_string();
            let (code, body) = http_request("POST", &format!("/api/v1/releases/{}/submit", rel_id), &host, port, Some(&sub_body), Some(tok)).await?;
            if code != 200 {
                println!("[x] Submit failed (HTTP {}): {}", code, body);
                return Ok(());
            }
            println!("      ✓ Release state transitioned to Submitted");

            // Step 5: Approve Release with Flight Director Role
            println!("\n[5/8] Approving release with Flight Director authorization...");
            let app_body = serde_json::json!({ "reason": "Flight Director authorization granted" }).to_string();
            let (code, body) = http_request("POST", &format!("/api/v1/releases/{}/approve", rel_id), &host, port, Some(&app_body), Some(tok)).await?;
            if code != 200 {
                println!("[x] Approval failed (HTTP {}): {}", code, body);
                return Ok(());
            }
            println!("      ✓ Release state transitioned to Approved");

            // Step 6: Run RAE Assurance Pipeline
            println!("\n[6/8] Running Release Assurance Engine (RAE) pipeline...");
            let (code, body) = http_request("POST", &format!("/api/v1/releases/{}/assure", rel_id), &host, port, Some("{}"), Some(tok)).await?;
            if code != 200 {
                println!("[x] Assurance failed (HTTP {}): {}", code, body);
                return Ok(());
            }
            let rep_json: serde_json::Value = serde_json::from_str(&body)?;
            let decision = rep_json.get("decision").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let score = rep_json.get("confidence_score").and_then(|v| v.as_i64()).unwrap_or(0);
            println!("      ✓ RAE Decision: {} (Confidence: {}%)", decision, score);
            println!("      ✓ Release state transitioned to Assured");

            // Step 7: Assess Deployment Readiness (DRA)
            println!("\n[7/8] Assessing Deployment Readiness Assessment (DRA)...");
            let (code, body) = http_request("POST", &format!("/api/v1/releases/{}/readiness", rel_id), &host, port, Some("{}"), Some(tok)).await?;
            if code != 200 {
                println!("[x] Readiness assessment failed (HTTP {}): {}", code, body);
                return Ok(());
            }
            let dra_json: serde_json::Value = serde_json::from_str(&body)?;
            let dra_decision = dra_json.get("decision").and_then(|v| v.as_str()).unwrap_or("Unknown");
            println!("      ✓ DRA Decision: {} (Deployment window validated)", dra_decision);
            println!("      ✓ Release state transitioned to Ready");

            // Step 8: Dispatch to Distributed Execution Engine (DEE)
            println!("\n[8/8] Dispatching to Distributed Execution Engine (DEE)...");
            let (code, body) = http_request("POST", &format!("/api/v1/releases/{}/deploy", rel_id), &host, port, Some("{}"), Some(tok)).await?;
            if code != 202 && code != 200 {
                println!("[x] Deployment dispatch failed (HTTP {}): {}", code, body);
                return Ok(());
            }
            let dep_json: serde_json::Value = serde_json::from_str(&body)?;
            let session_id = dep_json.get("session_id").and_then(|v| v.as_str()).unwrap_or("?");
            println!("      ✓ DEE Session Initialized: {}", session_id);
            println!("      ✓ Telecommand chunk packet streaming active to Standby Bank B");
            println!("\n🎉 Full Release Lifecycle (ROE -> RAE -> DRA -> DEE) successfully completed!");
        }
        "version" | "--version" | "-v" => {
            println!("PLATO Mission Control CLI v1.0.0 (Rust 2021 Edition)");
        }
        "--help" | "-h" | "help" => {
            println!("PLATO Operator Command-Line Interface");
            println!("Usage: plato-cli <COMMAND>\n");
            println!("Commands:");
            println!("  status              Check PLATO API connectivity and operational state");
            println!("  login [user] [pass] Authenticate operator and persist session token");
            println!("  health              Probe /health, /health/live, /health/ready endpoints");
            println!("  satellites          List active constellation assets and telemetry");
            println!("  orbits              Query SGP4 propagated ephemeris and ground stations");
            println!("  release-cycle [ver] Execute full ROE -> RAE -> DRA -> DEE release lifecycle");
            println!("  deployments         List active DEE deployment queue and packets");
            println!("  audit-verify        Verify cryptographic SHA-256 hash audit ledger chain");
            println!("  version             Display CLI version");
            println!("  help                Display this help message");
        }
        unknown => {
            println!("Unknown command: '{}'. Run `plato-cli --help` for usage.", unknown);
        }
    }

    Ok(())
}
