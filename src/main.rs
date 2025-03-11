use std::time::Duration;
use std::sync::Arc;

use clap::Parser;
use reqwest::{self, Client};
use tokio::task::JoinSet;
use tokio::sync::Semaphore;
use tokio::runtime::Builder;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]

struct Args {
    #[arg(short, long)]
    url: String,
    #[arg(short, long)]
    method: String,
    #[arg(short, long)]
    data: Option<String>,
    #[arg(short, long, default_value = "1")]
    number_of_requests: i32,
    #[arg(short, long, default_value = "1")]
    concurent_requests: i32,
    #[arg(short, long, default_value = "1")]
    tasks: i32,
    #[arg(short, long)]
    run_time: Option<i32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let number_of_threads = std::cmp::max(1, args.tasks as usize);
    let runtime = Builder::new_multi_thread()
        .worker_threads(number_of_threads)
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    runtime.block_on(async_main(args))
}

async fn async_main(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let Args { url, method, data, number_of_requests, concurent_requests, .. } = args;
    let sem = Arc::new(Semaphore::new(concurent_requests as usize));
    let client = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
    let mut set = JoinSet::new();
    for _ in 0..number_of_requests {
        let url = url.clone();
        let method = method.clone();
        let data = data.clone();
        let client = client.clone();
        let sem = sem.clone();
        set.spawn(async move {
            let _permit = sem.acquire().await;
            if let Err(_) = _permit {
                println!("Error: Semaphore acquire failed");
            }
            let res = request(&client,&url, &method, data).await;
            drop(_permit);
            match res {
                Ok(_) => {},
                Err(e) => println!("Error: {:?}", e),
            }
        });
    }
    while let Some(res) = set.join_next().await {
        if let Err(e) = res {
            eprintln!("Task failed: {:?}", e);
        }
    }
    Ok(())
}

async fn request(client: &Client,url: &str, method: &str, data: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let resp = match method {
        "GET" => {
            if let Some(_) = data {
                return Err(Box::<dyn std::error::Error>::from("GET method does not support data"));
            }
            let res = client.get(url).send().await?;
            res
        },
        "POST" => {
            let mut builder = client.post(url);
            if let Some(data) = data {
                builder = builder.body(data);
            }
            let res = builder.send().await?;
            res
        },
        _ => return Err(Box::<dyn std::error::Error>::from("Invalid HTTP method")),
    };
    let _ = resp.text().await?;
    Ok(())
}