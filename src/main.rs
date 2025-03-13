use rlimit::{getrlimit, setrlimit, Resource};
use std::process::exit;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use reqwest::{self, Client, Response};
use tokio::runtime::Builder;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

mod utils;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]

struct Args {
    #[arg(short, long)]
    url: String,
    #[arg(short, long)]
    method: String,
    #[arg(short = 'g', long)]
    headers: Option<String>,
    #[arg(short, long)]
    data: Option<String>,
    #[arg(short, long, default_value = "1")]
    concurent_requests: u32,
    #[arg(short, long, default_value = "1")]
    tasks: u8,
    #[arg(short, long, default_value = "60")]
    run_time: u32,
    #[arg(short, long)]
    sleep: Option<u128>,
    #[arg(short = 'l', long)]
    rate_limit: Option<u32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    utils::logging::log();
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
    let Args {
        url,
        method,
        data,
        concurent_requests,
        sleep,
        run_time,
        rate_limit,
        ..
    } = args;
    let sem = Arc::new(Semaphore::new(concurent_requests as usize));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        //.pool_max_idle_per_host(concurent_requests as usize)
        .tcp_nodelay(true)
        .build()?;

    if let Ok((soft, _)) = getrlimit(Resource::NOFILE) {
        if soft < concurent_requests as u64 {
            setrlimit(
                Resource::NOFILE,
                soft + (concurent_requests as u64) * 2,
                soft + (concurent_requests as u64) * 2,
            )?;
        }
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    let deadline = Instant::now() + Duration::from_secs(run_time as u64);
    let mut set = JoinSet::new();
    let mut rate_limiter =
        rate_limit.map(|r| tokio::time::interval(Duration::from_secs_f64(1.0 / r as f64)));
    loop {
        if Instant::now() > deadline {
            break;
        }

        if let Some(ref mut interval) = rate_limiter {
            interval.tick().await;
        }
        let url = url.clone();
        let method = method.clone();
        let data = data.clone();
        let client = client.clone();
        let sleep = sleep.clone();
        let sem = sem.clone();

        if set.len() >= concurent_requests as usize {
            if let Some(res) = set.join_next().await {
                if let Err(e) = res {
                    eprintln!("Task failed: {:?}", e);
                }
            }
        }

        set.spawn(async move {
            if let Some(sleep) = sleep {
                tokio::time::sleep(Duration::from_millis(sleep as u64)).await;
            }
            let _permit = sem.acquire().await;
            if let Err(_) = _permit {
                println!("Error: Semaphore acquire failed");
            }
            let res = request(&client, &url, &method, data).await;
            drop(_permit);
            match res {
                Ok(res) => {
                    //dbg!(res);
                    //println!("Status: {}", res.status());
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                }
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

async fn request(
    client: &Client,
    url: &str,
    method: &str,
    data: Option<String>,
) -> Result<Response, reqwest::Error> {
    let resp = match method {
        "GET" => {
            let res = client.get(url).send().await?;
            res
        }
        "POST" => {
            let mut builder = client.post(url);
            if let Some(data) = data {
                builder = builder.body(data);
            }
            let res = builder.send().await?;
            res
        }
        _ => panic!("Invalid HTTP method"),
    };

    Ok(resp)
}
