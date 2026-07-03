use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::time::{sleep, Duration};

const REPORT_FILE: &str = "./safecrack_report.md";
const MAX_ATTEMPTS_PER_SESSION: usize = 10;
const MAX_RETRIES: usize = 3;

struct TestResult {
    combo: String, status: String, details: String,
    latency_ms: u128, payload_bytes: usize, http_status: String,
}

async fn create_session(url: &str, id: usize) -> Result<Client, Box<dyn std::error::Error>> {
    let client = Client::builder().cookie_store(true).build()?;
    let name = format!("Browser_Sim_B{}", id);
    println!("[SESSION] Starting clean isolated 10-attempt window: {}", name);

    let mut token = HashMap::new();
    token.insert("action", "set_name");
    token.insert("name", name.as_str());

    let mut r = 0;
    loop {
        if client.post(url).form(&token).send().await.is_ok() { break; }
        r += 1; if r >= MAX_RETRIES { return Err("Init failed".into()); }
        sleep(Duration::from_millis(500)).await;
    }
    Ok(client)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 { std::process::exit(1); }
    let target_url: &str = &args[1];
    
    let mut delay_ms: u64 = 0;
    if let Some(idx) = args.iter().position(|x| x == "--delay") {
        if idx + 1 < args.len() { delay_ms = args[idx + 1].parse().unwrap_or(0); }
    }

    let p_a = vec!["red", "green", "blue"];
    let p_b = vec!["left", "middle", "right"];
    let p_c = vec!["0", "1", "2"];
    let p_d = vec!["alpha", "beta", "gamma"];

    let mut test_cases = Vec::new();
    let max_len = p_a.len().max(p_b.len()).max(p_c.len()).max(p_d.len());
    
    for i in 0..(max_len * max_len) {
        let a = p_a[(i / max_len) % p_a.len()];
        let b = p_b[i % p_b.len()];
        let c = p_c[(i / max_len) % p_c.len()];
        let d = p_d[((i / max_len) + (i % max_len)) % p_d.len()];
        test_cases.push((a, b, c, d, "Pairwise Matrix Rule".to_string()));
    }

    for a in &p_a {
        for b in &p_b {
            for c in &p_c {
                for d in &p_d {
                    let is_3w = a != &"red" && b == &"right" && d == &"alpha";
                    let is_4w = a == &"red" && b == &"right" && c == &"2" && d == &"gamma";
                    if is_3w || is_4w {
                        let lbl = if is_3w { "Augmented: 3-Way Core Interaction" } else { "Augmented: Strict 4-Way Glitch" };
                        test_cases.push((a, b, c, d, lbl.to_string()));
                    }
                }
            }
        }
    }

    let mut logs = Vec::new();
    let mut current_batch = 1;
    let mut client = create_session(target_url, current_batch).await?;
    let mut attempt_counter = 0;

    // Browser Default Starting State (when you click Start)
    let mut cur_a = "red";
    let mut cur_b = "left";
    let mut cur_c = "0";
    let mut cur_d = "alpha";

    println!("Running matrix sweep using Delta Browser Simulation...");

    for (a, b, c, d, label) in test_cases {
        if attempt_counter >= MAX_ATTEMPTS_PER_SESSION {
            current_batch += 1;
            client = create_session(target_url, current_batch).await?;
            attempt_counter = 0;
            // Reset browser memory back to safe defaults for the new session
            cur_a = "red"; cur_b = "left"; cur_c = "0"; cur_d = "alpha";
        }

        let start = Instant::now();
        let targets = vec![("A", a, &mut cur_a), ("B", b, &mut cur_b), ("C", c, &mut cur_c), ("D", d, &mut cur_d)];

        // Delta Clicker: Only execute a POST if the target differs from current state
        for (param, target_val, current_val) in targets {
            if target_val != *current_val {
                if delay_ms > 0 { sleep(Duration::from_millis(delay_ms)).await; }
                let mut form = HashMap::new();
                form.insert("action", "select");
                form.insert("param", param);
                form.insert("value", target_val);
                
                let _ = client.post(target_url).form(&form).send().await;
                *current_val = target_val; // Sync local browser memory
            }
        }

        if delay_ms > 0 { sleep(Duration::from_millis(delay_ms)).await; }
        let mut open_form = HashMap::new();
        open_form.insert("action", "add_attempt");

        if let Ok(res) = client.post(target_url).form(&open_form).send().await {
            let status = res.status().to_string();
            if let Ok(body) = res.text().await {
                attempt_counter += 1;
                let combo = format!("{} | {} | {} | {}", a, b, c, d);
                let size = body.len();
                let mut eval_status = "SECURELY_LOCKED".to_string();
                let mut diagnostics = "Safe remained locked".to_string();

                let doc = Html::parse_document(&body);
                if let Ok(sel) = Selector::parse(".display") {
                    for el in doc.select(&sel) {
                        let txt = el.text().collect::<Vec<_>>().join(" ").to_lowercase();
                        if !txt.contains("closed") {
                            if combo == "red | left | 0 | alpha" {
                                eval_status = "LEGITIMATE_CODE".to_string();
                                diagnostics = "Authorized standard route".to_string();
                            } else {
                                eval_status = "BUG_FOUND".to_string();
                                diagnostics = label.clone();
                            }
                        }
                    }
                }

                logs.push(TestResult { combo, status: eval_status, details: diagnostics, latency_ms: start.elapsed().as_millis(), payload_bytes: size, http_status: status });
            }
        }
    }

    let mut file = File::create(REPORT_FILE)?;
    writeln!(file, "# Production Safe Cracking Metrics & Comprehensive Delta Browser Report")?;
    writeln!(file, "\n| Combination Profile (A, B, C, D) | Status | Diagnostics | Latency | Size | HTTP |")?;
    writeln!(file, "| :--- | :--- | :--- | :--- | :--- | :--- |")?;
    for r in logs {
        writeln!(file, "| **{}** | `{}` | {} | {}ms | {} B | `{}` |", r.combo, r.status, r.details, r.latency_ms, r.payload_bytes, r.http_status)?;
    }
    println!("Sweep complete! Results written cleanly to: {}", REPORT_FILE);
    Ok(())
}

