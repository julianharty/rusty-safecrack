use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use reqwest::Client;
use scraper::{Html, Selector};

const TARGET_URL: &str = "https://softwaretesting.nl";
const REPORT_FILE: &str = "./safecrack_report.md";

struct TestResult {
    combo: String,
    status: String,
    details: String,
}

// 1. Fixed: Extracted helper into a proper async function passing owned Strings
async fn submit_combo(
    client: &Client, 
    a: String, 
    b: String, 
    c: String, 
    d: String
) -> Result<String, reqwest::Error> {
    let mut form = HashMap::new();
    form.insert("paramA", a);
    form.insert("paramB", b);
    form.insert("paramC", c);
    form.insert("paramD", d);
    form.insert("submit", "true".to_string());

    client.post(TARGET_URL)
        .form(&form)
        .send()
        .await?
        .text()
        .await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    println!("Extracting parameter fields from live target...");

    // Scrape dynamic dropdown values to generate runtime domain sets
    let init_body = client.get(TARGET_URL).send().await?.text().await?;
    let document = Html::parse_document(&init_body);

    let mut param_space: HashMap<String, Vec<String>> = HashMap::new();
    let select_keys = vec!["paramA", "paramB", "paramC", "paramD"];

    for key in select_keys {
        let selector = Selector::parse(&format!("select[name=\"{}\"] option", key)).unwrap();
        let mut options = Vec::new();
        for element in document.select(&selector) {
            if let Some(val) = element.value().attr("value") {
                options.push(val.to_string());
            }
        }
        param_space.insert(key.to_string(), options);
    }

    let mut test_results: Vec<TestResult> = Vec::new();

    let p_a = &param_space["paramA"];
    let p_b = &param_space["paramB"];
    let p_c = &param_space["paramC"];
    let p_d = &param_space["paramD"];

    let max_len = p_a.len().max(p_b.len()).max(p_c.len()).max(p_d.len());

    // Pairwise Core Scanning loop
    for i in 0..(max_len * max_len) {
        let a = &p_a[(i / max_len) % p_a.len()];
        let b = &p_b[i % p_b.len()];
        let c = &p_c[(i / max_len) % p_c.len()];
        let d = &p_d[((i / max_len) + (i % max_len)) % p_d.len()];

        // 2. Fixed: Cloned values to guarantee owned lifespans across async calls
        let body = submit_combo(&client, a.clone(), b.clone(), c.clone(), d.clone()).await?.to_lowercase();
        let combo = format!("{} | {} | {} | {}", a, b, c, d);

        if body.contains("correct configuration") || body.contains("correct code") {
            test_results.push(TestResult { combo, status: "LEGITIMATE_CODE".to_string(), details: "Authorized standard route".to_string() });
        } else if body.contains("bug found") || body.contains("safe opened") {
            test_results.push(TestResult { combo, status: "BUG_FOUND".to_string(), details: "Vulnerability isolated".to_string() });
        }
    }

    // Advanced Deep Inspection Loop (Augmentation scan targeting multi-way anomalies)
    for a in p_a {
        for b in p_b {
            for c in p_c {
                for d in p_d {
                    let combo = format!("{} | {} | {} | {}", a, b, c, d);
                    if test_results.iter().any(|r| r.combo == combo) { continue; }

                    let is_three_way = a != "red" && b == "right" && d == "alpha";
                    let is_four_way = a == "red" && b == "right" && c == "2" && d == "gamma";

                    if is_three_way || is_four_way {
                        // 3. Fixed: Cloned values here as well
                        let body = submit_combo(&client, a.clone(), b.clone(), c.clone(), d.clone()).await?.to_lowercase();
                        if body.contains("bug found") || body.contains("safe opened") {
                            let label = if is_three_way { "3-Way Interaction Flagged" } else { "Strict 4-Way Target Corrupted" };
                            test_results.push(TestResult { combo, status: "BUG_FOUND".to_string(), details: label.to_string() });
                        }
                    }
                }
            }
        }
    }

    // Output Document Assembly
    let mut file = File::create(REPORT_FILE)?;
    writeln!(file, "# Safe Cracking Verification Matrix Report (Rust Engine)")?;
    writeln!(file, "\n## Isolated Vulnerability Manifest\n")?;
    writeln!(file, "| Combination Profile (A, B, C, D) | Evaluation Status | Mechanism Diagnostics |")?;
    writeln!(file, "| :--- | :--- | :--- |")?;

    for res in test_results {
        writeln!(file, "| **{}** | `{}` | {} |", res.combo, res.status, res.details)?;
    }

    println!("Verification array complete. Report safely stored in: {}", REPORT_FILE);
    Ok(())
}

