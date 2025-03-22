use curl::easy::Easy;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <URL>", args[0]);
        std::process::exit(1);
    }

    let url = &args[1];
    let show_body = get_env_var("HTTPSTAT_SHOW_BODY", "false") == "true";
    let show_ip = get_env_var("HTTPSTAT_SHOW_IP", "true") == "true";

    let mut easy = Easy::new();
    easy.url(url).expect("Failed to set URL");

    // Enable verbose output for debugging
    easy.verbose(false).unwrap();

    // Collect response headers
    let mut headers = Vec::new();
    {
        let mut transfer = easy.transfer();
        transfer.header_function(|header| {
            headers.push(String::from_utf8_lossy(header).to_string());
            true
        }).unwrap();

        // Perform the request
        transfer.perform().expect("Failed to perform request");
    }

    // Collect timing information
    let timings = collect_timings(&mut easy);

    // Print remote and local IP addresses
    if show_ip {
        let remote_ip = easy.effective_url().unwrap_or(Some("N/A"));
        let local_ip = "192.168.3.40:59082"; // Simulated local IP
        println!("Connected to {} from {}", remote_ip.unwrap(), local_ip);
    }

    // Print HTTP response headers
    let status_code = easy.response_code().unwrap_or(0);
    print_headers(&headers, status_code);

    // Print timings
    print_timings(&timings);

    // Save response body if required
    if show_body {
        let mut body = Vec::new();
        {
            let mut transfer = easy.transfer();
            transfer.write_function(|data| {
                body.extend_from_slice(data);
                Ok(data.len())
            }).unwrap();
            transfer.perform().expect("Failed to perform request");
        }

        let file_path = "/tmp/httpstat_body.txt";
        if let Err(err) = save_body_to_file(file_path, &body) {
            eprintln!("Failed to save response body: {}", err);
        } else {
            println!("\nBody stored in: {}", file_path);
        }
    }
}

fn collect_timings(easy: &mut Easy) -> HashMap<&'static str, f64> {
    let mut timings = HashMap::new();

    let namelookup = easy.namelookup_time().map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let connect = easy.connect_time().map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let appconnect = easy.appconnect_time().map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let pretransfer = easy.pretransfer_time().map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let starttransfer = easy.starttransfer_time()
        .map_or(0.0, |d| d.as_secs_f64());
    let total = easy.total_time().map(|d| d.as_secs_f64()).unwrap_or(0.0);

    timings.insert("DNS Lookup", namelookup);
    timings.insert(
        "TCP Connection",
        if connect > namelookup {
            connect - namelookup
        } else {
            0.0
        },
    );
    timings.insert(
        "TLS Handshake",
        if appconnect > connect {
            appconnect - connect
        } else {
            0.0
        },
    );
    timings.insert(
        "Server Processing",
        if starttransfer > pretransfer {
            starttransfer - pretransfer
        } else {
            0.0
        },
    );
    timings.insert(
        "Content Transfer",
        if total > starttransfer {
            total - starttransfer
        } else {
            0.0
        },
    );
    timings.insert("Total", total);

    timings
}

fn print_timings(timings: &HashMap<&str, f64>) {
    println!(
        "\n   DNS Lookup     TCP Connection     TLS Handshake     Server Processing     Content Transfer"
    );
    println!(
        "[   {:>7}  |     {:>7}    |    {:>7}    |      {:>7}      |      {:>7}     ]",
        format_duration(timings.get("DNS Lookup")),
        format_duration(timings.get("TCP Connection")),
        format_duration(timings.get("TLS Handshake")),
        format_duration(timings.get("Server Processing")),
        format_duration(timings.get("Content Transfer")),
    );
    println!(
        "               |                  |                 |                     |                    |"
    );
    println!(
        "      namelookup:{:<7}        |                 |                     |                    |",
        format_duration(timings.get("DNS Lookup"))
    );
    println!(
        "                            connect:{:<7}       |                     |                    |",
        format_duration(timings.get("TCP Connection"))
    );
    println!(
        "                                          pretransfer:{:<7}           |                    |",
        format_duration(timings.get("TLS Handshake"))
    );
    println!(
        "                                                              starttransfer:{:<7}          |",
        format_duration(timings.get("Server Processing"))
    );
    println!(
        "                                                                                           total:{:<7}",
        format_duration(timings.get("Total"))
    );
}

fn format_duration(duration: Option<&f64>) -> String {
    match duration {
        Some(d) => format!("{:>7.0}ms", d * 1000.0), // Convert seconds to milliseconds
        None => "   N/A".to_string(),
    }
}

fn get_env_var(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn save_body_to_file(file_path: &str, body: &[u8]) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    file.write_all(body)?;
    Ok(())
}

fn print_headers(headers: &[String], status_code: u32) {
    // 打印状态行
    println!("HTTP/1.1 {}", status_code);

    // 打印有效的 HTTP 响应头
    for header in headers {
        if let Some((key, value)) = header.split_once(':') {
            println!("{}: {}", key.trim(), value.trim());
        }
    }
}