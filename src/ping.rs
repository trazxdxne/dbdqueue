use std::collections::HashMap;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ratatui::style::Color;

pub fn ping_aws_region(region_code: &str, timeout: Duration) -> Option<u32> {
    // 1. Try HTTP Keep-Alive ping to dynamodb.<region>.amazonaws.com/ping (Cloudping.info technique).
    // The first request performs DNS + TCP + TLS handshake (warming up the connection into the pool).
    // The second request is sent over the already-open HTTP Keep-Alive TLS connection,
    // measuring strictly 1 network round-trip time (RTT), matching ICMP ping accuracy while
    // preventing local TUN/proxy adapters from faking 0 ms TCP handshakes.
    let url = format!("https://dynamodb.{}.amazonaws.com/ping", region_code);
    let agent = ureq::builder()
        .timeout(timeout)
        .try_proxy_from_env(true)
        .build();

    // Warm-up request: establish connection and drain small body to return socket to pool
    if let Ok(resp) = agent.get(&url).call() {
        let _ = resp.into_string();

        // Timed Keep-Alive request: measures pure 1 RTT over active TLS stream
        let start = Instant::now();
        if let Ok(ping_resp) = agent.get(&url).call() {
            let _ = ping_resp.into_string();
            let elapsed = start.elapsed().as_millis() as u32;
            return Some(elapsed);
        }
    }

    // 2. Fallback to raw TCP connection if HTTPS request fails
    let host = format!("dynamodb.{}.amazonaws.com:443", region_code);
    let addrs = host.to_socket_addrs().ok()?;
    let addr = addrs.into_iter().next()?;
    
    let tcp_start = Instant::now();
    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => Some(tcp_start.elapsed().as_millis() as u32),
        Err(_) => None,
    }
}

pub fn measure_all_regions_ping() -> HashMap<String, u32> {
    let regions = crate::api::get_all_aws_regions();
    let results = Arc::new(Mutex::new(HashMap::new()));
    
    std::thread::scope(|s| {
        for reg in &regions {
            let reg_str = reg.to_string();
            let results_clone = Arc::clone(&results);
            s.spawn(move || {
                let timeout = Duration::from_millis(2500);
                if let Some(ms) = ping_aws_region(&reg_str, timeout)
                    && let Ok(mut map) = results_clone.lock()
                {
                    map.insert(reg_str, ms);
                }
            });
        }
    });
    
    results.lock().unwrap().clone()
}

pub fn color_for_ping(ms: Option<u32>) -> Color {
    match ms {
        Some(p) if p <= 80 => Color::Green,
        Some(p) if p < 250 => Color::Yellow,
        Some(_) => Color::Red,
        None => Color::DarkGray,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_for_ping() {
        assert_eq!(color_for_ping(Some(30)), Color::Green);
        assert_eq!(color_for_ping(Some(80)), Color::Green);
        assert_eq!(color_for_ping(Some(81)), Color::Yellow);
        assert_eq!(color_for_ping(Some(120)), Color::Yellow);
        assert_eq!(color_for_ping(Some(249)), Color::Yellow);
        assert_eq!(color_for_ping(Some(250)), Color::Red);
        assert_eq!(color_for_ping(Some(300)), Color::Red);
        assert_eq!(color_for_ping(None), Color::DarkGray);
    }

    #[test]
    fn test_all_regions_ping() {
        let pings = measure_all_regions_ping();
        for (reg, ms) in &pings {
            println!("{}: {} ms", reg, ms);
        }
    }
}
