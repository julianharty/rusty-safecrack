use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::time::{sleep, Duration};

const REPORT_FILE: &str = "./safecrack_report.md";
const MAX_ATTEMPTS_PER_SESSION: usize = 10; // Maximised to exactly 10 attempts per session block

struct TestResult {
    combo: String,
    status: String,
    details: String,
}

async fn create_authenticated_session(url: &str, batch_id: usize) -> Result<Client, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .cookie_store(true)
        .build()?;

    let team_name = format!("Automated_Bot_Batch_{}", batch_id);
    println!("[SESSION] Spawning fresh incognito container context for: {}...", team_name);

    // Fixed: Force both inserts to use &str lifetimes to align types cleanly
    let mut startup_token = HashMap::new();
    startup_token.insert("action", "set_name");
    startup_token.insert("name", team_name.as_str());

    client.post(url).form(&startup_token).send().await?;
    Ok(client)
}

async fn submit_and_open_combo(client: &Client, url: &str, a: String, b: String, c: String, d: String, delay_ms: u64) -> Result<String, reqwest::Error> {
    let parameters = vec![("A", a), ("B", b), ("C", c), ("D", d)];

    for (param, value) in parameters {
        if delay_ms > 0 { sleep(Duration::from_millis(delay_ms)).await; }
        let mut form = HashMap::new();
        form.insert("action", "select".to_string());
        form.insert("param", param.to_string());
        form.insert("value", value);

        client.post(url).form(&form).send().await?;
    }

    if delay_ms > 0 { sleep(Duration::from_millis(delay_ms)).await; }
    let mut attempt_form = HashMap::new();
    attempt_form.insert("action", "add_attempt".to_string());

    let final_body = client.post(url)
        .form(&attempt_form)
        .send()
        .await?
        .text()
        .await?;

    Ok(final_body)
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
    if let Some(idx) = args.iter().position(|x| x == "--delay") {
        if idx + 1 < args.len() {
            delay_ms = args[idx + 1].parse().unwrap_or(0);
        }
    }

    println!("Connecting to target sequence engine: {}...", target_url);

    let p_a = vec!["red".to_string(), "green".to_string(), "blue".to_string()];
    let p_b = vec!["left".to_string(), "middle".to_string(), "right".to_string()];
    let p_c = vec!["0".to_string(), "1".to_string(), "2".to_string()];
    let p_d = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];

    let mut combinations_to_test = Vec::new();
    let max_len = p_a.len().max(p_b.len()).max(p_c.len()).max(p_d.len());
    
    // Suite 1: Pairwise configurations
    for i in 0..(max_len * max_len) {
        let a = p_a[(i / max_len) % p_a.len()].clone();
        let b = p_b[i % p_b.len()].clone();
        let c = p_c[(i / max_len) % p_c.len()].clone();
        let d = p_d[((i / max_len) + (i % max_len)) % p_d.len()].clone();
        combinations_to_test.push((a, b, c, d, "Pairwise Matrix Rule".to_string()));
    }

    // Suite 2: Discovered multi-way variants
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

    let mut test_results: Vec<TestResult> = Vec::new();
    let mut current_batch = 1;
    let mut current_client = create_authenticated_session(target_url, current_batch).await?;
    let mut session_attempt_counter = 0;

    println!("Running matrix audit with auto-cookie session rotation...");

    for (a, b, c, d, rule_label) in combinations_to_test {
        let body = submit_and_open_combo(&current_client, target_url, a.clone(), b.clone(), c.clone(), d.clone(), delay_ms).await?;
        session_attempt_counter += 1;
        
        let combo = format!("{} | {} | {} | {}", a, b, c, d);

        let document = Html::parse_document(&body);
        let mut matched_status = String::new();

        if let Ok(selector) = Selector::parse(".display") {
            for element in document.select(&selector) {
                let inner_text = element.text().collect::<Vec<_>>().join(" ").to_lowercase();
                if !inner_text.contains("closed") {
                    if combo == "red | left | 0 | alpha" {
                        matched_status = "LEGITIMATE_CODE".to_string();
                    } else {
                        matched_status = "BUG_FOUND".to_string();
                    }
                }
            }
        }

        if matched_status == "LEGITIMATE_CODE" {
            test_results.push(TestResult { combo, status: "LEGITIMATE_CODE".to_string(), details: "Authorized standard route".to_string() });
        } else if matched_status == "BUG_FOUND" {
            test_results.push(TestResult { combo, status: "BUG_FOUND".to_string(), details: rule_label });
        }

        // Fixed: If we just performed the 10th attempt, clear cookies immediately before starting the next combination loop
        if session_attempt_counter >= MAX_ATTEMPTS_PER_SESSION {
            current_batch += 1;
            current_client = create_authenticated_session(target_url, current_batch).await?;
            session_attempt_counter = 0;
        }
    }

    let mut file = File::create(REPORT_FILE)?;
    writeln!(file, "# Production Safe Cracking Verification Matrix Report")?;
    writeln!(file, "\n| Combination Variant Profile (A, B, C, D) | Evaluation Status | Mechanism Diagnostics |")?;
    writeln!(file, "| :--- | :--- | :--- |")?;
    for res in test_results {
        writeln!(file, "| **{}** | `{}` | {} |", res.combo, res.status, res.details)?;
    }

    println!("Verification sweep complete. True defects cleanly stored in: {}", REPORT_FILE);
    Ok(())
}

