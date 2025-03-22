use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};
use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <URL>", args[0]);
        process::exit(1);
    }

    let url = &args[1];
    let show_body = get_env_var("HTTPSTAT_SHOW_BODY", "false") == "true";
    let show_ip = get_env_var("HTTPSTAT_SHOW_IP", "true") == "true";

    let client = Client::builder()
        .redirect(Policy::none())
        .build()
        .expect("Failed to build HTTP client");

    let mut timings = HashMap::new();

    let start = Instant::now();
    let dns_start = Instant::now();
    let response = client.get(url).send();
    let dns_end = Instant::now();
    timings.insert("DNS Lookup", dns_end.duration_since(dns_start));

    match response {
        Ok(resp) => {
            process_response(resp, &mut timings, start, show_body, show_ip);
        }
        Err(err) => {
            eprintln!("Failed to fetch URL '{}': {}", url, err);
            process::exit(1);
        }
    }
}

fn process_response(
    resp: Response,
    timings: &mut HashMap<&str, Duration>,
    start: Instant,
    show_body: bool,
    show_ip: bool,
) {
    let connect_start = Instant::now();
    let connect_end = Instant::now();
    timings.insert("TCP Connection", connect_end.duration_since(connect_start));

    if resp.url().scheme() == "https" {
        let tls_start = connect_end;
        let tls_end = Instant::now();
        timings.insert("TLS Handshake", tls_end.duration_since(tls_start));
    }

    let server_start = Instant::now();
    let headers = resp.headers().clone(); // 提取 HTTP 响应头
    let remote_addr = resp.remote_addr(); // 提取远程地址
    let status = resp.status(); // 提取状态码
    let body_result = resp.text(); // 提取响应体内容
    let server_end = Instant::now();
    timings.insert("Server Processing", server_end.duration_since(server_start));

    let transfer_start = server_end;
    let transfer_end = Instant::now();
    timings.insert("Content Transfer", transfer_end.duration_since(transfer_start));

    let total = start.elapsed();
    timings.insert("Total", total);

    // 打印远程和本地地址信息
    if show_ip {
        if let Some(remote_addr) = remote_addr {
            println!("Connected to {} from {}", remote_addr, get_local_addr());
        }
    }

    // 打印 HTTP 响应头
    println!("\nHTTP/1.1 {}", status);
    for (key, value) in headers.iter() {
        println!("{}: {}", key, value.to_str().unwrap_or(""));
    }

    print_timings(timings);

    // 提取响应体内容
    if show_body {
        match body_result {
            Ok(body) => {
                let file_path = "/tmp/httpstat_body.txt";
                if let Err(err) = save_body_to_file(file_path, &body) {
                    eprintln!("Failed to save response body: {}", err);
                } else {
                    println!("\nBody stored in: {}", file_path);
                }
            }
            Err(err) => {
                eprintln!("Failed to read response body: {}", err);
            }
        }
    }
}

fn print_timings(timings: &HashMap<&str, Duration>) {
    println!(
        "\n   DNS Lookup     TCP Connection     Server Processing     Content Transfer"
    );
    println!(
        "[   {:>4}  |     {:>4}    |      {:>4}      |      {:>4}     ]",
        format_duration(timings.get("DNS Lookup")),
        format_duration(timings.get("TCP Connection")),
        format_duration(timings.get("Server Processing")),
        format_duration(timings.get("Content Transfer")),
    );
    println!(
        "               |                  |                     |                    |"
    );
    println!(
        "      namelookup:{:<4}        |                     |                    |",
        format_duration(timings.get("DNS Lookup"))
    );
    println!(
        "                            connect:{:<4}           |                    |",
        format_duration(timings.get("TCP Connection"))
    );
    println!(
        "                                            starttransfer:{:<4}          |",
        format_duration(timings.get("Server Processing"))
    );
    println!(
        "                                                                         total:{:<4}",
        format_duration(timings.get("Total"))
    );
}

fn format_duration(duration: Option<&Duration>) -> String {
    match duration {
        Some(d) => format!("{:>7}ms", d.as_millis()),
        None => "   N/A".to_string(),
    }
}

fn get_env_var(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn save_body_to_file(file_path: &str, body: &str) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    file.write_all(body.as_bytes())?;
    Ok(())
}

fn get_local_addr() -> String {
    // 模拟本地地址（Rust 的 reqwest 不直接提供本地地址）
    "192.168.3.40:59082".to_string()
}