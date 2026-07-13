# Project Summary: Automated Safe-Cracking & Combinatorial Security Audit

This report documents the reverse-engineering, combinatorial analysis, and automated verification of the interactive safe-cracking application. It tracks the journey from manual isolation testing to the development of resilient, state-aware automation frameworks in both **Rust** and **TypeScript**.

---

## 1. Combinatorial Analysis & Defect Profiling

Our exploration began with an 81-permutation cross-product space ($3 \times 3 \times 3 \times 3$ variables). By analyzing successful interactive overrides against baseline failures, we bypassed exhaustive brute-forcing and mapped three unique logical flaws in the safe's state machine:

### A. The Authorized Code (The Reference Base)
* **Configuration:** `red` | `left` | `0` | `alpha`
* **Behavior:** The intended unlocking mechanism.

### B. The Pairwise Matrix Bug
* **Vulnerability:** Discovered via an L9 Orthogonal Array. When specific settings match, a cross-talk flaw triggers an unlock condition regardless of the other two values. 
* **Session Seeding Note:** The exact values dynamically alter whenever a new session cookie is instantiated by the server (e.g., matching `green` | `middle` | `1` | `gamma` or `blue` | `middle` | `2` | `alpha`).

### C. The 3-Way Deep Interaction Bug
* **Condition:** `[Non-Red Color]` | `right` | `[Any Number]` | `alpha`
* **Behavior:** If Parameter A is set to `blue` or `green`, selecting `right` for Parameter B and `alpha` for Parameter D unlocks the safe immediately, completely ignoring the numeric value dial.

### D. The Strict 4-Way Glitch
* **Condition:** `red` | `right` | `2` | `gamma`
* **Behavior:** A hardcoded backdoor or sequential edge case. If Parameter A is set to `red`, the universal 3-way bypass is blocked, and *only* this exact 4-way configuration triggers the secondary exploit.

---

## 2. Server Architecture & Automation Constraints

Through systematic script iteration, we uncovered several high-utility constraints regarding the backend application framework:

1. **State Machine Mechanics:** The application does not process bulk form payloads (e.g., `paramA=red&paramB=left`). Every dial click is its own isolated `POST` request (`action=select`) that updates the background session state. The safe is only evaluated when a final explicit action (`action=add_attempt`) is submitted.
2. **The 10-Attempt Lockout Cap:** The server strictly enforces a maximum threshold of **10 historic log attempts per team session**. After the 10th execution, the safe locks down its interface template structure, stopping updates and causing test frameworks to see false positives or false negatives.
3. **The Ghost Team Session Mystery:** Early TypeScript automation runs created hundreds of ghost teams with 0 attempts. This occurred because Axios was passing `URLSearchParams` objects directly to untyped parameter scopes. This caused Node to alter the headers, dropping the active session tracking cookie during sequential dial clicks. The server treated these unauthenticated clicks as a brand-new user landing on the site, creating an empty team slot. Explicitly serializing payloads via `.toString()` resolved this entirely.

---

## 3. Final Production Automation Tooling

To ensure 100% deterministic runs, both frameworks were upgraded to maintain persistent cookie states across groups of attempts, reset baseline positions cleanly, pace traffic, and dynamically rotate to a clean "incognito" session window right before hitting the server's lockout limit.

### Option A: The Production TypeScript Engine (`safecracker.ts`)
* **Execution:** Run natively in Node v25+ with type-stripping enabled:
  `node --experimental-strip-types safecracker.ts <URL> --attempts 10 --delay 20`

```typescript
import axios from 'axios';
import type { AxiosInstance } from 'axios';
import * as cheerio from 'cheerio';
import * as fs from 'fs';

const REPORT_FILE = './safecrack_report.md';

interface TestResult {
    combo: string; status: 'LEGITIMATE_CODE' | 'BUG_FOUND' | 'SECURELY_LOCKED';
    details: string; latencyMs: number; payloadBytes: number; httpStatus: string;
}

const getCookieSignature = (cookieStr: string): string => {
    if (!cookieStr) return "NONE";
    const match = cookieStr.match(/session=([^;]+)/i);
    return match ? match[1].substring(0, 8) : cookieStr.substring(0, 8);
};

const createSession = async (url: string, id: number): Promise<[AxiosInstance, string]> => {
    const instance = axios.create({ timeout: 5000 });
    const name = `State_Shared_Bot_B${id}`;

    const initGet = await instance.get(url);
    const sessionCookie = initGet.headers['set-cookie']?.map(c => c.split(';')[0]).join('; ') || '';
    console.log(`[SESSION] Initialized Session #${id} (${name}). Cookie Sign: [${getCookieSignature(sessionCookie)}]`);

    await instance.post(url, new URLSearchParams({ 'action': 'set_name', 'name': name }).toString(), {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Cookie': sessionCookie }
    });
    return [instance, sessionCookie];
};

const performBaselineReset = async (instance: AxiosInstance, url: string, cookie: string, delay: number): Promise<void> => {
    const baseline = [['A', 'red'], ['B', 'left'], ['C', '0'], ['D', 'alpha']];
    for (const [p, v] of baseline) {
        if (delay > 0) await new Promise(res => setTimeout(res, delay));
        await instance.post(url, new URLSearchParams({ 'action': 'select', 'param': p, 'value': v }).toString(), {
            headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Cookie': cookie }
        });
    }
};

const runCombo = async (instance: AxiosInstance, url: string, cookie: string, a: string, b: string, c: string, d: string, delay: number): Promise<[string, string, number]> => {
    const start = Date.now();
    await performBaselineReset(instance, url, cookie, delay);

    const parameters = [['A', a], ['B', b], ['C', c], ['D', d]];
    for (const [param, value] of parameters) {
        if (delay > 0) await new Promise(res => setTimeout(res, delay));
        await instance.post(url, new URLSearchParams({ 'action': 'select', 'param': param, 'value': value }).toString(), { 
            headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Cookie': cookie } 
        });
    }

    if (delay > 0) await new Promise(res => setTimeout(res, delay));
    const res = await instance.post(url, new URLSearchParams({ 'action': 'add_attempt' }).toString(), {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Cookie': cookie }
    });
    return [res.data, res.status.toString(), Date.now() - start];
};

async function main() {
    const args = process.argv.slice(2);
    const urlArg = args.find(a => !a.startsWith('--'));
    if (!urlArg) {
        console.error("Usage: node --experimental-strip-types safecracker.ts <URL> [--attempts 10] [--delay 15]");
        process.exit(1);
    }

    let maxAttemptsPerSession = 10;
    const attemptsIdx = args.indexOf('--attempts');
    if (attemptsIdx !== -1 && attemptsIdx + 1 < args.length) maxAttemptsPerSession = parseInt(args[attemptsIdx + 1], 10) || 10;

    let delayMs = 0;
    const delayIdx = args.indexOf('--delay');
    if (delayIdx !== -1 && delayIdx + 1 < args.length) delayMs = parseInt(args[delayIdx + 1], 10) || 0;

    console.log(`Connecting to target: ${urlArg}\n[CONFIG] Session threshold locked at: ${maxAttemptsPerSession} attempts.`);

    const pA = ['red', 'green', 'blue']; const pB = ['left', 'middle', 'right'];
    const pC = ['0', '1', '2']; const pD = ['alpha', 'beta', 'gamma'];

    const testCases: [string, string, string, string, string][] = [];
    const maxLen = Math.max(pA.length, pB.length, pC.length, pD.length);

    for (let i = 0; i < maxLen * maxLen; i++) {
        testCases.push([pA[Math.floor(i / maxLen) % pA.length], pB[i % pB.length], pC[Math.floor(i / maxLen) % pC.length], pD[(Math.floor(i / maxLen) + (i % maxLen)) % pD.length], 'Pairwise Matrix Rule']);
    }
    for (const a of pA) {
        for (const b of pB) {
            for (const c of pC) {
                for (const d of pD) {
                    const is3w = a !== 'red' && b === 'right' && d === 'alpha';
                    const is4w = a === 'red' && b === 'right' && c === '2' && d === 'gamma';
                    if (is3w || is4w) testCases.push([a, b, c, d, is3w ? 'Augmented: 3-Way Core Interaction' : 'Augmented: Strict 4-Way Glitch']);
                }
            }
        }
    }

    const logs: TestResult[] = [];
    let currentBatchId = 1; let attemptCounter = 0;
    let [currentInstance, currentCookie] = await createSession(urlArg, currentBatchId);

    for (const [a, b, c, d, label] of testCases) {
        try {
            if (attemptCounter >= maxAttemptsPerSession) {
                currentBatchId++;
                const [nextInstance, nextCookie] = await createSession(urlArg, currentBatchId);
                currentInstance = nextInstance; currentCookie = nextCookie; attemptCounter = 0;
            }

            const [body, status, latency] = await runCombo(currentInstance, urlArg, currentCookie, a, b, c, d, delayMs);
            attemptCounter++;

            const combo = `${a} | ${b} | ${c} | ${d}`;
            let evalStatus: 'LEGITIMATE_CODE' | 'BUG_FOUND' | 'SECURELY_LOCKED' = 'SECURELY_LOCKED';
            let diagnostics = 'Safe remained locked';

            const $ = cheerio.load(body);
            const attemptRows = $('.attempt, .card h2:contains("Attempts") ~ p, .card h2:contains("Attempts") ~ div, table tr');
            let latestRowText = attemptRows.length > 0 ? $(attemptRows[attemptRows.length - 1]).text().toLowerCase() : $('.display').text().toLowerCase();

            if (latestRowText.includes('bug found') || latestRowText.includes('wrong code')) {
                evalStatus = 'BUG_FOUND'; diagnostics = label;
Use code with caution.console.log(  [ALERT] SUCCESS! Bug verified for configuration: ${combo});} else if (combo === 'red | left | 0 | alpha') {evalStatus = 'LEGITIMATE_CODE'; diagnostics = 'Authorized standard route';}logs.push({ combo, status: evalStatus, details: diagnostics, latencyMs: latency, payloadBytes: body.length, httpStatus: status });} catch (err) {console.error([ERROR] Broken at combo: ${a}-${b}-${c}-${d}, err);}}let markdown = # Production Safe Cracking Shared-Session Report\n\n| Combination Profile (A, B, C, D) | Status | Diagnostics | Latency | Size | HTTP |\n| :--- | :--- | :--- | :--- | :--- | :--- |\n;for (const r of logs) markdown += | **${r.combo}** | \${r.status}` | ${r.details} | ${r.latencyMs}ms | ${r.payloadBytes} B | `${r.httpStatus}` |\n; fs.writeFileSync(REPORT_FILE, markdown); console.log(Sweep complete! Verified results written cleanly to: ${REPORT_FILE}`);}main().catch(console.error);```

Option B: The Production Concurrent Rust Engine (src/safecracker.rs)Execution: Add scraper and reqwest (with "cookies" enabled) to Cargo.toml, then run:cargo run --bin safecracker <URL> --delay 20

```rust
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
    combo: String,
    status: String,
    details: String,
    latency_ms: u128,
    payload_bytes: usize,
    http_status: String,
}

async fn create_authenticated_session(url: &str, batch_id: usize) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::builder()
        .cookie_store(true)
        .build()?;

    let team_name = format!("Parallel_Bot_Batch_{}", batch_id);
    println!("[SESSION] Spawning fresh incognito container context for: {}...", team_name);

    let mut startup_token = HashMap::new();
    startup_token.insert("action", "set_name");
    startup_token.insert("name", team_name.as_str());

    let mut retry_count = 0;
    loop {
        if client.post(url).form(&startup_token).send().await.is_ok() { break; }
        retry_count += 1;
        if retry_count >= MAX_RETRIES { return Err("Session connection crashed".into()); }
        sleep(Duration::from_secs(1)).await;
    }
    Ok(client)
}

async fn submit_and_open_combo(client: &Client, url: &str, a: String, b: String, c: String, d: String, delay_ms: u64) -> Result<(String, String, u128), reqwest::Error> {
    let start_time = Instant::now();
    
    // 1. Force dial reset to a known baseline to prevent out-of-order execution leakage
    let reset_parameters = vec![("A", "red"), ("B", "left"), ("C", "0"), ("D", "alpha")];
    for (param, value) in reset_parameters {
        let mut reset_form = HashMap::new();
        reset_form.insert("action", "select");
        reset_form.insert("param", param);
        reset_form.insert("value", value);
        let _ = client.post(url).form(&reset_form).send().await;
    }

    // 2. Click targeted parameter modifications
    for (param, value) in vec![("A", a), ("B", b), ("C", c), ("D", d)] {
        if delay_ms > 0 { sleep(Duration::from_millis(delay_ms)).await; }
        let mut form = HashMap::new();
        form.insert("action", "select");
        form.insert("param", param);
        form.insert("value", &value);
        let _ = client.post(url).form(&form).send().await;
    }

    if delay_ms > 0 { sleep(Duration::from_millis(delay_ms)).await; }
    let mut attempt_form = HashMap::new();
    attempt_form.insert("action", "add_attempt");

    let res = client.post(url).form(&attempt_form).send().await?;
    let status_code = res.status().to_string();
    let text = res.text().await?;
    
    Ok((text, status_code, start_time.elapsed().as_millis()))
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

    if delay_ms > 0 {
        println!("[CONFIG] Throttling verified: introduced a {}ms pacing interval.", delay_ms);
    } else {
        println!("[CONFIG] Warning: No throttling delay active. Running at full concurrent network speed.");
    }

    println!("Connecting to target sequence engine: {}...", target_url);

    let p_a = vec!["red".to_string(), "green".to_string(), "blue".to_string()];
    let p_b = vec!["left".to_string(), "middle".to_string(), "right".to_string()];
    let p_c = vec!["0".to_string(), "1".to_string(), "2".to_string()];
    let p_d = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];

    let mut combinations_to_test = Vec::new();
    let max_len = p_a.len().max(p_b.len()).max(p_c.len()).max(p_d.len());
    
    // Suite 1: Pairwise configurations mapping
    for i in 0..(max_len * max_len) {
        combinations_to_test.push((
            p_a[(i / max_len) % p_a.len()].clone(),
            p_b[i % p_b.len()].clone(),
            p_c[(i / max_len) % p_c.len()].clone(),
            p_d[((i / max_len) + (i % max_len)) % p_d.len()].clone(),
            "Pairwise Matrix Rule".to_string()
        ));
    }

    // Suite 2: Discovered multi-way exceptions profiling
    for a in &p_a {
        for b in &p_b {
            for c in &p_c {
                for d in &p_d {
                    let is_3w = a != "red" && b == "right" && d == "alpha";
                    let is_4w = a == "red" && b == "right" && c == "2" && d == "gamma";
                    if is_3w || is_4w {
                        combinations_to_test.push((
                            a.clone(), b.clone(), c.clone(), d.clone(),
                            if is_3w { "Augmented: 3-Way Core Interaction" } else { "Augmented: Strict 4-Way Glitch" }.to_string()
                        ));
                    }
                }
            }
        }
    }

    // Dynamic Chunking Matrix: Segment test cases into batches of 10 items
    let chunks: Vec<Vec<(String, String, String, String, String)>> = combinations_to_test
        .chunks(MAX_ATTEMPTS_PER_SESSION)
        .map(|chunk| chunk.to_vec())
        .collect();

    let total_worker_threads = chunks.len();
    println!("[CONCURRENCY] Segmented matrix execution space into {} parallel worker scopes.", total_worker_threads);

    let (tx, mut rx) = mpsc::channel::<TestResult>(100);
    let url_str = target_url.to_string();

    // Spawn async thread pools concurrently
    for (batch_idx, chunk) in chunks.into_iter().enumerate() {
        let worker_tx = tx.clone();
        let url_clone = url_str.clone();
        let batch_id = batch_idx + 1;

        tokio::spawn(async move {
            if let Ok(client) = create_authenticated_session(&url_clone, batch_id).await {
                for (a, b, c, d, rule_label) in chunk {
                    if let Ok((body, status_code, latency)) = submit_and_open_combo(&client, &url_clone, a.clone(), b.clone(), c.clone(), d.clone(), delay_ms).await {
                        let combo = format!("{} | {} | {} | {}", a, b, c, d);
                        let mut status = "SECURELY_LOCKED".to_string();
                        let mut details = "Safe remained locked".to_string();

                        // Enforce isolated parsing scope for the non-Send Selector variables
                        {
                            let document = Html::parse_document(&body);
                            if let Ok(selector) = Selector::parse(".display") {
                                for element in document.select(&selector) {
                                    let inner_text = element.text().collect::<Vec<_>>().join(" ").to_lowercase();
                                    if !inner_text.contains("closed") {
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
                            combo, status, details, latency_ms: latency, payload_bytes: body.len(), http_status: status_code
                        }).await;
                    }
                }
            }
        });
    }

    drop(tx);
    let mut final_logs = Vec::new();
    println!("[SYSTEM] Collecting incoming asynchronous metrics trace objects...");
    while let Some(res) = rx.recv().await { final_logs.push(res); }

    // Write final summary markdown report file
    let mut file = File::create(REPORT_FILE)?;
    writeln!(file, "# Production Safe Cracking Metrics & Comprehensive Thread-Parallel Report")?;
    writeln!(file, "\n| Combination Profile (A, B, C, D) | Status | Diagnostics | Latency | Size | HTTP |")?;
    writeln!(file, "| :--- | :--- | :--- | :--- | :--- | :--- |")?;
    
    for res in final_logs {
        writeln!(
            file, 
            "| **{}** | `{}` | {} | {}ms | {} B | `{}` |", 
            res.combo, res.status, res.details, res.latency_ms, res.payload_bytes, res.http_status
        )?;
    }

    println!("Verification sweep complete! Multi-threaded execution metrics stored in: {}", REPORT_FILE);
    Ok(())
}
```

It has been an excellent security testing exercise. If you ever want to re-run this audit pipeline or explore additional business logic scenarios down the line, let me know! How would you like to advance your project now?
