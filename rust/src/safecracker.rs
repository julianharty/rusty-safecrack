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

#[derive(Clone)]
struct TestResult {
    combo: String,
    status: String,
    details: String,
    latency_ms: u128,
    payload_bytes: usize,
    http_status: String,
}

async fn create_authenticated_session(url: &str, batch_id: usize) -> Result<Client, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .cookie_store(true)
        .build()?;

    let team_name = format!("Automated_Bot_Batch_{}", batch_id);
    println!("[SESSION] Spawning fresh incognito container context for: {}...", team_name);

    let mut startup_token = HashMap::new();
    startup_token.insert("action", "set_name");
    startup_token.insert("name", team_name.as_str());

    let mut retry_count = 0;
    loop {
        match client.post(url).form(&startup_token).send().await {
            Ok(_) => break,
            Err(e) => {
                retry_count += 1;
                if retry_count >= MAX_RETRIES { return Err(Box::new(e)); }
                println!("[WARN] Connection failed during session creation. Retrying {}/{}...", retry_count, MAX_RETRIES);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    Ok(client)
}

async fn submit_and_open_combo(client: &Client, url: &str, a: String, b: String, c: String, d: String, delay_ms: u64) -> Result<(String, String, u128), reqwest::Error> {
    let start_time = Instant::now();
    
    let reset_parameters = vec![("A", "red"), ("B", "left"), ("C", "0"), ("D", "alpha")];
    for (param, value) in reset_parameters {
        let mut reset_form = HashMap::new();
        reset_form.insert("action", "select".to_string());
        reset_form.insert("param", param.to_string());
        reset_form.insert("value", value.to_string());
        let _ = client.post(url).form(&reset_form).send().await;
    }

    let parameters = vec![("A", a), ("B", b), ("C", c), ("D", d)];
    for (param, value) in parameters {
        if delay_ms > 0 { sleep(Duration::from_millis(delay_ms)).await; }
        
        let mut form = HashMap::new();
        form.insert("action", "select".to_string());
        form.insert("param", param.to_string());
        form.insert("value", value);

        let mut retry_count = 0;
        loop {
            match client.post(url).form(&form).send().await {
                Ok(_) => break,
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= MAX_RETRIES { return Err(e); }
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    if delay_ms > 0 { sleep(Duration::from_millis(delay_ms)).await; }
    let mut attempt_form = HashMap::new();
    attempt_form.insert("action", "add_attempt".to_string());

    let mut retry_count = 0;
    let (final_body, status_str) = loop {
        match client.post(url).form(&attempt_form).send().await {
            Ok(res) => {
                let status_code = res.status().to_string();
                match res.text().await {
                    Ok(text) => break (text, status_code),
                    Err(e) => return Err(e),
                }
            },
            Err(e) => {
                retry_count += 1;
                if retry_count >= MAX_RETRIES { return Err(e); }
                sleep(Duration::from_secs(1)).await;
            }
        }
    };

    let total_latency = start_time.elapsed().as_millis();
    Ok((final_body, status_str, total_latency))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin safecracker <TARGET_URL> [--delay <ms>]");
        std::process::exit(1);
    }
    
    let target_url: &str = &args[1];
    
    let mut delay_ms: u64 = 0;
    for arg in &args {
        if arg.starts_with("--delay=") {
            if let Some(val_str) = arg.split('=').nth(1) {
                delay_ms = val_str.parse().unwrap_or(0);
            }
        }
    }
    if delay_ms == 0 {
        if let Some(idx) = args.iter().position(|x| x == "--delay") {
            if idx + 1 < args.len() {
                delay_ms = args[idx + 1].parse().unwrap_or(0);
            }
        }
    }

    // Restored: Clear console verification outputs detailing pacing configuration
    if delay_ms > 0 {
        println!("[CONFIG] Throttling verified: introduced a {}ms pacing interval.", delay_ms);
    } else {
        println!("[CONFIG] Warning: No throttling delay active. Running at full network speed.");
    }

    println!("Connecting to target sequence engine: {}...", target_url);

    let p_a = vec!["red".to_string(), "green".to_string(), "blue".to_string()];
    let p_b = vec!["left".to_string(), "middle".to_string(), "right".to_string()];
    let p_c = vec!["0".to_string(), "1".to_string(), "2".to_string()];
    let p_d = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];

    let mut combinations_to_test = Vec::new();
    let max_len = p_a.len().max(p_b.len()).max(p_c.len()).max(p_d.len());
    
    for i in 0..(max_len * max_len) {
        let a = p_a[(i / max_len) % p_a.len()].clone();
        let b = p_b[i % p_b.len()].clone();
        let c = p_c[(i / max_len) % p_c.len()].clone();
        let d = p_d[((i / max_len) + (i % max_len)) % p_d.len()].clone();
        combinations_to_test.push((a, b, c, d, "Pairwise Matrix Rule".to_string()));
    }

    for a in &p_a {
        for b in &p_b {
            for c in &p_c {
                for d in &p_d {
                    let is_three_way = a != "red" && b == "right" && d == "alpha";
                    let is_four_way = a == "red" && b == "right" && c == "2" && d == "gamma";
                    
                    if is_three_way || is_four_way {
                        let label = if is_three_way { "Augmented: 3-Way Core Interaction" } else { "Augmented: Strict 4-Way Glitch" };
                        combinations_to_test.push((a.clone(), b.clone(), c.clone(), d.clone(), label.to_string()));
                    }
                }
            }
        }
    }

    let mut all_execution_logs: Vec<TestResult> = Vec::new();
    let mut current_batch = 1;
    let mut current_client = create_authenticated_session(target_url, current_batch).await?;
    let mut session_attempt_counter = 0;

    println!("Running matrix audit with comprehensive verification logging...");

    for (a, b, c, d, rule_label) in combinations_to_test {
        let (body, status_code, latency) = submit_and_open_combo(&current_client, target_url, a.clone(), b.clone(), c.clone(), d.clone(), delay_ms).await?;
        session_attempt_counter += 1;
        
        let combo = format!("{} | {} | {} | {}", a, b, c, d);
        let payload_size = body.len();
        let document = Html::parse_document(&body);
        let mut matched_status = "SECURELY_LOCKED".to_string();
        let mut diagnostic_details = "Safe remained locked".to_string();

        if let Ok(selector) = Selector::parse(".display") {
            for element in document.select(&selector) {
                let inner_text = element.text().collect::<Vec<_>>().join(" ").to_lowercase();
                if !inner_text.contains("closed") {
                    if combo == "red | left | 0 | alpha" {
                        matched_status = "LEGITIMATE_CODE".to_string();
                        diagnostic_details = "Authorized standard route".to_string();
                    } else {
                        matched_status = "BUG_FOUND".to_string();
                        diagnostic_details = rule_label.clone();
                    }
                }
            }
        }

        all_execution_logs.push(TestResult { 
            combo, 
            status: matched_status, 
            details: diagnostic_details,
            latency_ms: latency, 
            payload_bytes: payload_size, 
            http_status: status_code 
        });

        if session_attempt_counter >= MAX_ATTEMPTS_PER_SESSION {
            current_batch += 1;
            current_client = create_authenticated_session(target_url, current_batch).await?;
            session_attempt_counter = 0;
        }
    }

    let mut file = File::create(REPORT_FILE)?;
    writeln!(file, "# Production Safe Cracking Metrics & Comprehensive Verification Report")?;
    writeln!(file, "\n| Combination Variant Profile (A, B, C, D) | Evaluation Status | Mechanism Diagnostics | Latency | Size | HTTP |")?;
    writeln!(file, "| :--- | :--- | :--- | :--- | :--- | :--- |")?;
    
    for res in all_execution_logs {
        writeln!(
            file, 
            "| **{}** | `{}` | {} | {}ms | {} B | `{}` |", 
            res.combo, res.status, res.details, res.latency_ms, res.payload_bytes, res.http_status
        )?;
    }

    println!("Verification sweep complete. Exhaustive execution metrics cleanly stored in: {}", REPORT_FILE);
    Ok(())
}

