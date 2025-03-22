use std::collections::HashMap;
use std::env;
use std::time::{Duration, Instant};
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::redirect::Policy;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <URL>", args[0]);
        process::exit(1);
    }

    let url = &args[1];
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
            let connect_start = dns_end;
            let connect_end = Instant::now();
            timings.insert("TCP Connection", connect_end.duration_since(connect_start));

            if resp.url().scheme() == "https" {
                let tls_start = connect_end;
                let tls_end = Instant::now();
                timings.insert("TLS Handshake", tls_end.duration_since(tls_start));
            }

            let server_start = Instant::now();
            let body = resp.text();
            let server_end = Instant::now();
            timings.insert("Server Processing", server_end.duration_since(server_start));

            let transfer_start = server_end;
            let transfer_end = Instant::now();
            timings.insert("Content Transfer", transfer_end.duration_since(transfer_start));

            let total = start.elapsed();
            timings.insert("Total", total);

            print_timings(&timings);
            if let Ok(body) = body {
                println!("\nResponse Body:\n{}", body);
            }
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    }
}

fn print_timings(timings: &HashMap<&str, Duration>) {
    println!(
        "  DNS Lookup   TCP Connection   TLS Handshake   Server Processing   Content Transfer"
    );
    println!(
        "[{:>10} | {:>10} | {:>10} | {:>10} | {:>10}]",
        format_duration(timings.get("DNS Lookup")),
        format_duration(timings.get("TCP Connection")),
        format_duration(timings.get("TLS Handshake")),
        format_duration(timings.get("Server Processing")),
        format_duration(timings.get("Content Transfer")),
    );
    println!(
        "Total: {}",
        format_duration(timings.get("Total"))
    );
}

fn format_duration(duration: Option<&Duration>) -> String {
    match duration {
        Some(d) => format!("{:>7}ms", d.as_millis()),
        None => "   N/A".to_string(),
    }
}