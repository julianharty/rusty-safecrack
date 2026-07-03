use std::collections::HashMap;
use std::env;
use reqwest::Client;

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
        std::process::exit(1);
    }
    let target_url = &args[1];

    let client = Client::builder()
        .cookie_store(true)
        .build()?;

    // 1. Log in to establish the session
    let mut startup_token = HashMap::new();
    startup_token.insert("action", "set_name");
    startup_token.insert("name", "Diagnostic Bot");

    client.post(target_url).form(&startup_token).send().await?;

    println!("------------------------------------------------------------");
    println!("DIAGNOSTIC RUN: Executing exactly one test combination...");
    println!("------------------------------------------------------------");

    // 2. Submit a single baseline combination
    let body_sample = submit_combo(
        &client, 
        target_url, 
        "red".to_string(), 
        "middle".to_string(), 
        "0".to_string(), 
        "beta".to_string()
    ).await?;

    // 3. Print the raw server feedback to console
    println!("{}", body_sample);
    println!("------------------------------------------------------------");
    println!("DIAGNOSTIC RUN COMPLETE.");
    println!("------------------------------------------------------------");

    Ok(())
}

