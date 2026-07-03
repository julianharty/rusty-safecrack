use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

const REPORT_FILE: &str = "./safecrack_report.md";
const MAX_ATTEMPTS_PER_SESSION: usize = 10; 
const MAX_RETRIES: usize = 3;

#[derive(Clone)]
struct TestResult {
    combo: String, status: String, details: String,
    latency_ms: u128, payload_bytes: usize, http_status: String,
}

async fn create_authenticated_session(url: &str, batch_id: usize) -> Result<(Client, String), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::builder().cookie_store(true).build()?;
    let name = format!("Parallel_Bot_B{}", batch_id); // Shorter, reliable session name
    println!("[SESSION] Starting isolated worker context: {}", name);

    let mut token = HashMap::new();
    token.insert("action", "set_name");
    token.insert("name", name.as_str());

    let mut retries = 0;
    loop {
        if client.post(url).form(&token).send().await.is_ok() { break; }
        retries += 1;
        if retries >= MAX_RETRIES { return Err("Session registration failed".into()); }
        sleep(Duration::from_secs(1)).await;
    }
    Ok((client, name))
}

async fn submit_and_open_combo(client: &Client, url: &str, a: &str, b: &str, c: &str, d: &str, delay: u64) -> Result<(String, String, u128), reqwest::Error> {
    let start = Instant::now();
    
    // Step 1: Enforce baseline reset
    for (p, v) in vec![("A", "red"), ("B", "left"), ("C", "0"), ("D", "alpha")] {
        let mut form = HashMap::new();
        form.insert("action", "select"); form.insert("param", p); form.insert("value", v);
        let _ = client.post(url).form(&form).send().await;
    }

    // Step 2: Input target values
    for (p, v) in vec![("A", a), ("B", b), ("C", c), ("D", d)] {
        if delay > 0 { sleep(Duration::from_millis(delay)).await; }
        let mut form = HashMap::new();
        form.insert("action", "select"); form.insert("param", p); form.insert("value", v);
        
        let mut r = 0;
        loop {
            if client.post(url).form(&form).send().await.is_ok() { break; }
            r += 1; if r >= MAX_RETRIES { break; }
            sleep(Duration::from_millis(500)).await;
        }
    }

    if delay > 0 { sleep(Duration::from_millis(delay)).await; }
    let mut open_form = HashMap::new();
    open_form.insert("action", "add_attempt");

    let mut r = 0;
    let (body, status) = loop {
        match client.post(url).form(&open_form).send().await {
            Ok(res) => {
                let code = res.status().to_string();
                if let Ok(txt) = res.text().await { break (txt, code); }
            },
            Err(_) => {
                r += 1; if r >= MAX_RETRIES { break ("".to_string(), "500".to_string()); }
                sleep(Duration::from_millis(500)).await;
            }
        }
    };

    Ok((body, status, start.elapsed().as_millis()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 { std::process::exit(1); }
    let target_url = &args[1];
    
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

    let chunks: Vec<Vec<(&str, &str, &str, &str, String)>> = test_cases
        .chunks(MAX_ATTEMPTS_PER_SESSION).map(|c| c.to_vec()).collect();

    let (tx, mut rx) = mpsc::channel::<TestResult>(100);
    let url_str = target_url.to_string();

    for (batch_idx, chunk) in chunks.into_iter().enumerate() {
        let worker_tx = tx.clone();
        let url_clone = url_str.clone();
        let batch_id = batch_idx + 1;

        tokio::spawn(async move {
            if let Ok((client, _)) = create_authenticated_session(&url_clone, batch_id).await {
                for (a, b, c, d, rule_label) in chunk {
                    if let Ok((body, status_code, latency)) = submit_and_open_combo(&client, &url_clone, a, b, c, d, delay_ms).await {
                        let combo = format!("{} | {} | {} | {}", a, b, c, d);
                        let size = body.len();
                        let mut status = "SECURELY_LOCKED".to_string();
                        let mut details = "Safe remained locked".to_string();

                        {
                            let doc = Html::parse_document(&body);
                            if let Ok(sel) = Selector::parse(".display") {
                                for el in doc.select(&sel) {
                                    let txt = el.text().collect::<Vec<_>>().join(" ").to_lowercase();
                                    if !txt.contains("closed") {
                                        if combo == "red | left | 0 | alpha" {
                                            status = "LEGITIMATE_CODE".to_string();
                                            details = "Authorized standard route".to_string();
                                        } else {
                                            status = "BUG_FOUND".to_string();
                                            details = rule_label.clone();
                                        }
                                    }
                                }
                            }
                        }

                        let _ = worker_tx.send(TestResult {
                            combo, status, details, latency_ms: latency, payload_bytes: size, http_status: status_code,
                        }).await;
                    }
                }
            }
        });
    }

    drop(tx);
    let mut logs = Vec::new();
    while let Some(res) = rx.recv().await { logs.push(res); }

    let mut file = File::create(REPORT_FILE)?;
    writeln!(file, "# Production Safe Cracking Metrics & Comprehensive Thread-Parallel Report")?;
    writeln!(file, "\n| Combination Profile (A, B, C, D) | Status | Diagnostics | Latency | Size | HTTP |")?;
    writeln!(file, "| :--- | :--- | :--- | :--- | :--- | :--- |")?;
    for r in logs {
        writeln!(file, "| **{}** | `{}` | {} | {}ms | {} B | `{}` |", r.combo, r.status, r.details, r.latency_ms, r.payload_bytes, r.http_status)?;
    }
    println!("Sweep complete! Results written cleanly to: {}", REPORT_FILE);
    Ok(())
}

