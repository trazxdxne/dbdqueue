use std::collections::HashMap;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ratatui::style::Color;

pub fn ping_aws_region(region_code: &str, timeout: Duration) -> Option<u32> {
    // 1. Try end-to-end TLS / HTTPS HEAD request to force the connection through any TUN / transparent proxy.
    // When a proxy or TUN adapter (WinTun / Sing-box / Hiddify / Clash) is active, raw TCP SYN packets
    // are answered locally in < 1ms (0ms ping). A TLS handshake cannot be faked locally and requires
    // full round-trip communication with the AWS datacenter.
    let url = format!("https://dynamodb.{}.amazonaws.com", region_code);
    let agent = ureq::builder()
        .timeout(timeout)
        .try_proxy_from_env(true)
        .build();

    let start = Instant::now();
    let res = agent.head(&url).call();
    let elapsed = start.elapsed().as_millis() as u32;

    match res {
        Ok(_) | Err(ureq::Error::Status(_, _)) => {
            // Even HTTP 400/403/404 confirms end-to-end TLS handshake and RTT to AWS
            Some(elapsed)
        }
        Err(_) => {
            // Fallback to raw TCP connection if HTTPS request fails
            let host = format!("dynamodb.{}.amazonaws.com:443", region_code);
            let addrs = host.to_socket_addrs().ok()?;
            let addr = addrs.into_iter().next()?;
            
            let tcp_start = Instant::now();
            match TcpStream::connect_timeout(&addr, timeout) {
                Ok(_) => Some(tcp_start.elapsed().as_millis() as u32),
                Err(_) => None,
            }
        }
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
                if let Some(ms) = ping_aws_region(&reg_str, timeout) {
                    if let Ok(mut map) = results_clone.lock() {
                        map.insert(reg_str, ms);
                    }
                }
            });
        }
    });
    
    let res = results.lock().unwrap().clone();
    res
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
}
