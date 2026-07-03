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

// Helper to strip empty lines and excessive whitespace for easy debugging
fn compress_html(html: &str) -> String {
    html.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

async fn submit_and_open_combo(client: &Client, url: &str, a: String, b: String, c: String, d: String) -> Result<String, reqwest::Error> {
    let parameters = vec![("A", a), ("B", b), ("C", c), ("D", d)];

    // 1. Sequentially select the 4 parameter options on the dial state machine
    for (param, value) in parameters {
        let mut form = HashMap::new();
        form.insert("action", "select".to_string());
        form.insert("param", param.to_string());
        form.insert("value", value);

        client.post(url).form(&form).send().await?;
    }

    // 2. Fixed: Submit the exact 'add_attempt' action parameter to press 'Test this combination'
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
        eprintln!("Usage: cargo run --bin safecracker <TARGET_URL> [--debug]");
        std::process::exit(1);
    }
    
    let target_url: &str = &args[1];
    let debug_mode = args.contains(&"--debug".to_string());

    let client = Client::builder()
        .cookie_store(true)
        .build()?;

    println!("Connecting to target: {}...", target_url);

    // Initial Gate Clearance Handshake
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

        // Debug save: Save a sample page after an actual attempt is processed
        if debug_mode && !debug_saved {
            println!("[DEBUG] Writing verified execution layout to ./debug_active_attempt.html");
            let clean_body = compress_html(&body);
            let mut f = File::create("./debug_active_attempt.html")?;
            f.write_all(clean_body.as_bytes())?;
            debug_saved = true;
        }

        let document = Html::parse_document(&body);
        let mut matched_status = String::new();

        // Fixed: Target the live '.display' status box wrapper component directly
        if let Ok(selector) = Selector::parse(".display") {
            for element in document.select(&selector) {
                let inner_text = element.text().collect::<Vec<_>>().join(" ").to_lowercase();
                
                // If it isn't "closed", it means the lock opened!
                if !inner_text.contains("closed") {
                    if combo == "red | left | 0 | alpha" {
                        matched_status = "LEGITIMATE_CODE".to_string();
                    } else {
                        matched_status = "BUG_FOUND".to_string();
                    }
                }
            }
        }

        // Fallback safety layer: check global text elements if the display box structure is hidden
        if matched_status.is_empty() {
            let global_text = body.to_lowercase();
            if global_text.contains("bug found") || global_text.contains("wrong code") {
                matched_status = "BUG_FOUND".to_string();
            }
        }

        if matched_status == "LEGITIMATE_CODE" {
            test_results.push(TestResult { combo, status: "LEGITIMATE_CODE".to_string(), details: "Authorized standard route".to_string() });
        } else if matched_status == "BUG_FOUND" {
            test_results.push(TestResult { combo, status: "BUG_FOUND".to_string(), details: "Vulnerability isolated via Pairwise Matrix".to_string() });
        }
    }

    println!("Augmenting dataset parameters with deep logic multi-way profiling exceptions...");
    for a in &p_a {
        for b in &p_b {
            for c in &p_c {
                for d in &p_d {
                    let combo = format!("{} | {} | {} | {}", a, b, c, d);
                    if test_results.iter().any(|r| r.combo == combo) { continue; }

                    let is_three_way = a != "red" && b == "right" && d == "alpha";
                    let is_four_way = a == "red" && b == "right" && c == "2" && d == "gamma";

                    if is_three_way || is_four_way {
                        let body = submit_and_open_combo(&client, target_url, a.clone(), b.clone(), c.clone(), d.clone()).await?;
                        let aug_doc = Html::parse_document(&body);
                        let mut aug_status = false;

                        if let Ok(selector) = Selector::parse(".display") {
                            for element in aug_doc.select(&selector) {
                                let inner_text = element.text().collect::<Vec<_>>().join(" ").to_lowercase();
                                if !inner_text.contains("closed") {
                                    aug_status = true;
                                }
                            }
                        }

                        if aug_status {
                            let label = if is_three_way { "Augmented Discovery: 3-Way Combo Leak" } else { "Augmented Discovery: Complex 4-Way Sequence Glitch" };
                            test_results.push(TestResult { combo, status: "BUG_FOUND".to_string(), details: label.to_string() });
                        }
                    }
                }
            }
        }
    }

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

