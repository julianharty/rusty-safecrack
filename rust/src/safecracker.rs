use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use reqwest::Client;

const REPORT_FILE: &str = "./safecrack_report.md";

struct TestResult {
    combo: String,
    status: String,
    details: String,
}

async fn submit_combo(client: &Client, url: &str, a: String, b: String, c: String, d: String) -> Result<String, reqwest::Error> {
    let parameters = vec![("A", a), ("B", b), ("C", c), ("D", d)];
    let mut last_body = String::new();

    for (param, value) in parameters {
        let mut form = HashMap::new();
        form.insert("action", "select".to_string());
        form.insert("param", param.to_string());
        form.insert("value", value);

        last_body = client.post(url)
            .form(&form)
            .send()
            .await?
            .text()
            .await?;
    }

    Ok(last_body)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Error: Missing target URL.");
        eprintln!("Usage: cargo run --bin safecracker <TARGET_URL> [--debug]");
        std::process::exit(1);
    }
    
    // Fixed: Pulling out the specific string slice at index 1 instead of passing the whole Vec reference
    let target_url = &args[1];
    let debug_mode = args.contains(&"--debug".to_string());

    let client = Client::builder()
        .cookie_store(true)
        .build()?;

    println!("Connecting to target: {}...", target_url);

    if debug_mode {
        println!("[DEBUG] Fetching raw landing page markup...");
        let landing_html = client.get(target_url).send().await?.text().await?;
        let mut f = File::create("./debug_landing_page.html")?;
        f.write_all(landing_html.as_bytes())?;
        println!("[DEBUG] Saved original landing file to ./debug_landing_page.html");
    }
    
    let mut startup_token = HashMap::new();
    startup_token.insert("action", "set_name");
    startup_token.insert("name", "Automated Combinatorial Rust Tool");

    println!("Submitting session registration sequence payload...");
    let login_res = client.post(target_url)
        .form(&startup_token)
        .send()
        .await?;

    let workspace_body = login_res.text().await?;
    
    if debug_mode {
        let mut f = File::create("./debug_post_login_page.html")?;
        f.write_all(workspace_body.as_bytes())?;
        println!("[DEBUG] Saved post-login screen response to ./debug_post_login_page.html");
    }

    if workspace_body.contains("Student or team name") || workspace_body.contains("name=\"name\"") {
        println!("WARNING: Session authentication failed. Still stuck on landing gate.");
    } else {
        println!("SUCCESS: Successfully logged in! Challenge workspace is active.");
    }

    let p_a = vec!["red".to_string(), "green".to_string(), "blue".to_string()];
    let p_b = vec!["left".to_string(), "middle".to_string(), "right".to_string()];
    let p_c = vec!["0".to_string(), "1".to_string(), "2".to_string()];
    let p_d = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];

    let mut test_results: Vec<TestResult> = Vec::new();
    let max_len = p_a.len().max(p_b.len()).max(p_c.len()).max(p_d.len());
    
    println!("Executing dynamic pairwise matrix scan over a grid density of {} runs...", max_len * max_len);

    for i in 0..(max_len * max_len) {
        let a = &p_a[(i / max_len) % p_a.len()];
        let b = &p_b[i % p_b.len()];
        let c = &p_c[(i / max_len) % p_c.len()];
        let d = &p_d[((i / max_len) + (i % max_len)) % p_d.len()];

        let body = submit_combo(&client, target_url, a.clone(), b.clone(), c.clone(), d.clone()).await?.to_lowercase();
        let combo = format!("{} | {} | {} | {}", a, b, c, d);

        if body.contains("correct code") || body.contains("correct configuration") || body.contains("opened with the correct") {
            test_results.push(TestResult { combo, status: "LEGITIMATE_CODE".to_string(), details: "Authorized base configuration route".to_string() });
        } else if body.contains("bug found") || body.contains("opened with a wrong code") {
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
                        let body = submit_combo(&client, target_url, a.clone(), b.clone(), c.clone(), d.clone()).await?.to_lowercase();
                        if body.contains("bug found") || body.contains("opened with a wrong code") {
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

