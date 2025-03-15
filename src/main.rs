use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::{Duration, Instant};

use utils::logging;

use clap::Parser;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::{self, Client, Error, Response};
use rlimit::{getrlimit, setrlimit, Resource};
use serde_json::Value;
use tokio::runtime::Builder;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;
use tokio::time;

mod utils;

#[derive(Parser, Debug)]
#[command(name = "gau_rs", version = "0.1", about = "A stress test tool")]
struct Args {
    #[arg(short, long)]
    url: String,
    #[arg(short = 'v', long, default_value = "DEFAULT")]
    title: String,
    #[arg(short, long)]
    method: String,
    #[arg(short = 'G', long, help="This should be a JSON string")]
    headers: Option<String>,
    #[arg(short, long, help="POST data")]
    data: Option<String>,
    #[arg(short, long, default_value = "1", help = "Number of concurrent requests")]
    concurent_requests: u32,
    #[arg(short, long, default_value = "1", help = "Number of tokio tasks to spawn")]
    tasks: u8,
    #[arg(short, long, default_value = "60", help = "The duration of the test in seconds")]
    run_time: u32,
    #[arg(short, long, help = "Sleep time in milliseconds between requests")]
    sleep: Option<u128>,
    #[arg(short = 'l', long, help = "Rate limit requests per second")]
    rate_limit: Option<f64>,
    #[arg(
        long = "redirect-policy",
        value_parser = parse_redirect_policy,
        default_value = "Default",
        help = r#"Sets the redirect policy. This controls the reqwest redirect behavior. 
        Currently, custom policies are not supported in this tool.
        
        Options:
          - None:       Do not follow redirects.
          - Default:    Follow up to 10 redirects (default behavior).
          - Limit=<N>:  Follow up to N redirects (e.g., Limit=5)."#
    )]
    redirect_policy: RedirectPolicy,
}

#[derive(Debug, Clone)]
enum RedirectPolicy {
    None,
    Default,
    Limit(u32),
}

impl FromStr for RedirectPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(RedirectPolicy::None),
            "default" => Ok(RedirectPolicy::Default),
            _ if s.starts_with("limit=") => s[6..]
                .parse::<u32>()
                .map(RedirectPolicy::Limit)
                .map_err(|_| "Invalid number for Limit (expected Limit=<number>)".to_string()),
            _ => Err(format!(
                "Invalid value '{}'. Expected: None, Default, Limit=<number>",
                s
            )),
        }
    }
}

// Custom parser function for clap
fn parse_redirect_policy(s: &str) -> Result<RedirectPolicy, String> {
    RedirectPolicy::from_str(s)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let number_of_threads = std::cmp::max(1, args.tasks as usize);

    let runtime = Builder::new_multi_thread()
        .worker_threads(number_of_threads)
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    if let Ok((soft, _)) = getrlimit(Resource::NOFILE) {
        if soft < args.concurent_requests as u64 {
            setrlimit(
                Resource::NOFILE,
                soft + (args.concurent_requests as u64) * utils::constants::MULTIPLIER,
                soft + (args.concurent_requests as u64) * utils::constants::MULTIPLIER,
            )?;
        }
    }
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
        title,
        headers,
        redirect_policy,
        ..
    } = args;
    let sem = Arc::new(Semaphore::new(concurent_requests as usize));
    let latency_mutex: Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));
    let request_per_second_mutex: Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));
    let request_per_second_counter = Arc::new(Mutex::new(0.0));
    let req_byte_size_arr_mutex: Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));

    let start_time = SystemTime::now();

    let request_counter_mutex = Arc::new(Mutex::new(0));
    let errors_mutex = Arc::new(Mutex::new(0));
    let non2xx_mutex = Arc::new(Mutex::new(0));

    let client = construct_gau_client(redirect_policy, concurent_requests)?;

    tokio::time::sleep(Duration::from_secs(2)).await;
    let deadline = Instant::now() + Duration::from_secs(run_time as u64);
    let mut set = JoinSet::new();
    let mut rate_limiter =
        rate_limit.map(|r| tokio::time::interval(Duration::from_secs_f64(1.0 / r as f64)));
    if let Some(ref mut interval) = rate_limiter {
        interval.tick().await;
    }

    let counter_clone = request_per_second_counter.clone();
    let history_clone = request_per_second_mutex.clone();
    tokio::spawn(async move {
        let start = time::Instant::now() + time::Duration::from_secs(1);
        let mut ticker = time::interval_at(start, Duration::from_secs(1));

        loop {
            ticker.tick().await; // Now it ticks only after 1 second
            let mut count = counter_clone.lock().await;
            let mut history = history_clone.lock().await;
            history.push(*count); // Store count
            println!("Requests this second: {}", *count);
            *count = 0.0; // Reset counter for the next second
        }
    });

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
        let req_byte_size_arr_mutex = req_byte_size_arr_mutex.clone();
        let sem = sem.clone();
        let headers = headers.clone();
        let latency_mutex = latency_mutex.clone();
        let request_per_second_counter = request_per_second_counter.clone();
        let request_counter = request_counter_mutex.clone();
        let errors = errors_mutex.clone();
        let non2xx = non2xx_mutex.clone();

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
            {
                let mut counter = request_per_second_counter.lock().await;
                *counter += 1.0;
            }
            let start = Instant::now();
            let res = request(&client, &url, &method, data, headers).await;
            let elapsed = start.elapsed().as_secs_f64();
            drop(_permit);
            {
                let mut latency = latency_mutex.lock().await;
                latency.push(elapsed);
            };

            match res {
                Ok(res) => {
                    let status_code = res.status();
                    let b = res.bytes().await.unwrap();
                    let mut req_byte_size_arr = req_byte_size_arr_mutex.lock().await;
                    req_byte_size_arr.push(b.len() as f64);
                    let mut counter = request_counter.lock().await;
                    *counter += 1;
                    if status_code.as_u16() < 200 || status_code.as_u16() >= 300 {
                        let mut non2xx = non2xx.lock().await;
                        *non2xx += 1;
                    }
                }
                Err(e) => {
                    let mut errors = errors.lock().await;
                    *errors += 1;
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
    let latency = latency_mutex.lock().await;
    let latency_vec = latency.clone();
    let latency_histogram = logging::PerformanceStats::from_data(latency_vec);
    // dbg!(latency_histogram);

    let history = request_per_second_mutex.lock().await;
    let history_vec = history.clone();
    let history_histogram = logging::PerformanceStats::from_data(history_vec);
    // dbg!(history_histogram);

    let req_byte_size = req_byte_size_arr_mutex.lock().await;
    let req_byte_size_vec = req_byte_size.clone();
    let req_byte_size_histogram = logging::PerformanceStats::from_data(req_byte_size_vec);
    // dbg!(req_byte_size_histogram);
    //
    let req = request_counter_mutex.lock().await.clone();
    let results = logging::Results::new(
        title,
        url,
        history_histogram,
        req,
        latency_histogram,
        req_byte_size_histogram,
        run_time,
        *errors_mutex.lock().await,
        0,
        start_time,
        SystemTime::now(),
        concurent_requests,
        *non2xx_mutex.lock().await,
    );
    dbg!(results);
    Ok(())
}

async fn request(
    client: &Client,
    url: &str,
    method: &str,
    data: Option<String>,
    headers: Option<String>,
) -> Result<Response, reqwest::Error> {
    let header = parse_from_json_string_headers(headers);
    let resp = match method {
        "GET" => {
            let res = client.get(url).headers(header).send().await?;
            res
        }
        "POST" => {
            let mut builder = client.post(url);
            if let Some(data) = data {
                builder = builder.body(data);
            }
            let res = builder.headers(header).send().await?;
            res
        }
        _ => panic!("Invalid HTTP method"),
    };
    Ok(resp)
}

fn parse_from_json_string_headers(headers: Option<String>) -> HeaderMap {
    if headers.is_none() {
        return HeaderMap::new();
    }
    let headers_json = serde_json::from_str(headers.unwrap().as_str());
    let mut header_map = HeaderMap::new();

    if let Some(Value::Object(map)) = headers_json.ok() {
        for (key, value) in map {
            if let Value::String(val) = value {
                if let (Ok(name), Ok(header_val)) = (
                    HeaderName::from_bytes(key.as_bytes()),
                    HeaderValue::from_str(&val),
                ) {
                    header_map.insert(name, header_val);
                }
            }
        }
    }
    header_map
}

fn construct_gau_client(
    redirect_policy: RedirectPolicy,
    _: u32,
) -> Result<Client, Error> {
    let mut client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        //.pool_max_idle_per_host(concurent_requests as usize)
        .tcp_nodelay(true);

    client = match redirect_policy {
        RedirectPolicy::None => client.redirect(Policy::none()),
        RedirectPolicy::Default => client.redirect(Policy::default()),
        RedirectPolicy::Limit(limit) => client.redirect(Policy::limited(limit as usize)),
    };

    client.build()
}
