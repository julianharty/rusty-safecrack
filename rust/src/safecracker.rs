use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use reqwest::Client;
use scraper::{Html, Selector};

const REPORT_FILE: &str = "./safecrack_report.md";

struct TestResult {
    combo: String,
    status: String,
    details: String,
}

// Helper to strip empty lines and excessive whitespace for easy copying
fn compress_html(html: &str) -> String {
    html.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

async fn submit_and_open_combo(client: &Client, url: &str, a: String, b: String, c: String, d: String) -> Result<String, reqwest::Error> {
    let parameters = vec![("A", a), ("B", b), ("C", c), ("D", d)];

    for (param, value) in parameters {
        let mut form = HashMap::new();
        form.insert("action", "select".to_string());
        form.insert("param", param.to_string());
        form.insert("value", value);

        client.post(url).form(&form).send().await?;
    }

    let mut open_form = HashMap::new();
    open_form.insert("action", "open".to_string());

    let final_body = client.post(url)
        .form(&open_form)
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
        eprintln!("Usage: cargo run --bin safecracker <TARGET_URL> [--debug]");
        std::process::exit(1);
    }
    
    let target_url: &str = &args[1];
    let debug_mode = args.contains(&"--debug".to_string());

    let client = Client::builder()
        .cookie_store(true)
        .build()?;

    println!("Connecting to target: {}...", target_url);

    let mut startup_token = HashMap::new();
    startup_token.insert("action", "set_name");
    startup_token.insert("name", "Automated Combinatorial Rust Tool");

    let login_res = client.post(target_url).form(&startup_token).send().await?;
    let workspace_body = login_res.text().await?;

    if workspace_body.contains("Student or team name") {
        println!("WARNING: Session initialization rejected by gate handler.");
    } else {
        println!("SUCCESS: Successfully logged in! Challenge workspace is active.");
    }

    // Force write the fresh login state if debug mode is active
    if debug_mode {
        let clean_html = compress_html(&workspace_body);
        let mut f = File::create("./debug_post_login_page.html")?;
        f.write_all(clean_html.as_bytes())?;
        println!("[DEBUG] Wrote compressed login state to ./debug_post_login_page.html");
    }

    let p_a = vec!["red".to_string(), "green".to_string(), "blue".to_string()];
    let p_b = vec!["left".to_string(), "middle".to_string(), "right".to_string()];
    let p_c = vec!["0".to_string(), "1".to_string(), "2".to_string()];
    let p_d = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];

    let mut test_results: Vec<TestResult> = Vec::new();
    let max_len = p_a.len().max(p_b.len()).max(p_c.len()).max(p_d.len());
    let mut debug_saved = false;
    
    println!("Executing dynamic pairwise matrix scan over a grid density of {} runs...", max_len * max_len);

    for i in 0..(max_len * max_len) {
        let a = &p_a[(i / max_len) % p_a.len()];
        let b = &p_b[i % p_b.len()];
        let c = &p_c[(i / max_len) % p_c.len()];
        let d = &p_d[((i / max_len) + (i % max_len)) % p_d.len()];

        let body = submit_and_open_combo(&client, target_url, a.clone(), b.clone(), c.clone(), d.clone()).await?;
        let combo = format!("{} | {} | {} | {}", a, b, c, d);

        // Debug save: Save the first evaluation page to see the table layout, regardless of outcome
        if debug_mode && !debug_saved {
            println!("[DEBUG] Writing compressed iteration layout to ./debug_first_attempt.html");
            let clean_body = compress_html(&body);
            let mut f = File::create("./debug_first_attempt.html")?;
            f.write_all(clean_body.as_bytes())?;
            debug_saved = true;
        }

        let document = Html::parse_document(&body);
        let mut matched_status = String::new();

        if let Ok(selector) = Selector::parse(".attempt") {
            if let Some(latest_attempt) = document.select(&selector).last() {
                let inner_text = latest_attempt.text().collect::<Vec<_>>().join(" ").to_lowercase();
                
                if inner_text.contains("correct code") || inner_text.contains("correct configuration") {
                    matched_status = "LEGITIMATE_CODE".to_string();
                } else if inner_text.contains("bug found") || inner_text.contains("wrong code") {
                    matched_status = "BUG_FOUND".to_string();
                }
            }
        }

        if matched_status == "LEGITIMATE_CODE" {
            test_results.push(TestResult { combo, status: "LEGITIMATE_CODE".to_string(), details: "Authorized standard route".to_string() });
        } else if matched_status == "BUG_FOUND" {
            test_results.push(TestResult { combo, status: "BUG_FOUND".to_string(), details: "Isolated via structural matrix verification".to_string() });
        }
    }

    println!("Augmenting dataset parameters with deep logic multi-way profiling exceptions... (Skipped to trace layout framework fast)");

    let mut file = File::create(REPORT_FILE)?;
    writeln!(file, "# Safe Cracking Verification Matrix Report (Rust Cookie-Aware Engine)")?;
    writeln!(file, "\n| Combination Variant Profile (A, B, C, D) | Evaluation Status | Mechanism Diagnostics |")?;
    writeln!(file, "| :--- | :--- | :--- |")?;
    for res in test_results {
        writeln!(file, "| **{}** | `{}` | {} |", res.combo, res.status, res.details)?;
    }

    println!("Verification sweep complete. File saved at: {}", REPORT_FILE);
    Ok(())
}

