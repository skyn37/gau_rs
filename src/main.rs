use std::collections::HashMap;
use clap;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::get("https://httpbin.org/ip").await?;
    let body = resp.text().await?;
    let json = serde_json::from_str::<HashMap<String, String>>(&body)?;
    println!("{json:#?}");
    Ok(())
}