use std::process::Command;
use serde::Deserialize;

#[derive(Deserialize)]
struct AwsIpRanges {
    prefixes: Vec<AwsPrefix>,
    ipv6_prefixes: Vec<AwsIpv6Prefix>,
}

#[derive(Deserialize)]
struct AwsPrefix {
    ip_prefix: String,
    region: String,
}

#[derive(Deserialize)]
struct AwsIpv6Prefix {
    ipv6_prefix: String,
    region: String,
}

fn fetch_aws_ip_ranges(regions_to_block: &[String]) -> Result<(Vec<String>, Vec<String>), String> {
    let url = "https://ip-ranges.amazonaws.com/ip-ranges.json";
    let resp = ureq::get(url)
        .set("User-Agent", "curl/8.7.1")
        .call()
        .map_err(|e| e.to_string())?;

    let body = resp.into_string().map_err(|e| e.to_string())?;
    let aws_data: AwsIpRanges = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    
    let mut blocked_ips_v4 = Vec::new();
    for prefix in aws_data.prefixes {
        if regions_to_block.contains(&prefix.region) {
            blocked_ips_v4.push(prefix.ip_prefix);
        }
    }
    
    let mut blocked_ips_v6 = Vec::new();
    for prefix in aws_data.ipv6_prefixes {
        if regions_to_block.contains(&prefix.region) {
            blocked_ips_v6.push(prefix.ipv6_prefix);
        }
    }
    
    Ok((blocked_ips_v4, blocked_ips_v6))
}

#[cfg(not(windows))]
pub fn update_firewall(selected_aws_regions: Option<&[String]>) {
    let all_regions = crate::api::get_all_aws_regions();
    let mut script = String::new();
    
    script.push_str("#!/bin/bash\n");
    // Cleanup old hosts file entries from previous versions of dbdqueue
    script.push_str("sed -i '/# --- DBD REGION CHANGER START ---/,/# --- DBD REGION CHANGER END ---/d' /etc/hosts\n");
    
    // Remove existing iptables rules
    script.push_str("iptables -D OUTPUT -m set --match-set dbdqueue_block dst -j REJECT 2>/dev/null\n");
    script.push_str("ip6tables -D OUTPUT -m set --match-set dbdqueue_block_v6 dst -j REJECT 2>/dev/null\n");
    // Also remove the old udp/icmp specific rules if they exist from a previous version
    script.push_str("iptables -D OUTPUT -p udp -m set --match-set dbdqueue_block dst -j REJECT 2>/dev/null\n");
    script.push_str("iptables -D OUTPUT -p icmp -m set --match-set dbdqueue_block dst -j REJECT 2>/dev/null\n");
    script.push_str("ip6tables -D OUTPUT -p udp -m set --match-set dbdqueue_block_v6 dst -j REJECT 2>/dev/null\n");
    script.push_str("ip6tables -D OUTPUT -p icmpv6 -m set --match-set dbdqueue_block_v6 dst -j REJECT 2>/dev/null\n");

    match selected_aws_regions {
        Some(selected) if selected.len() < all_regions.len() => {
            println!("\x1b[94mFetching AWS IP ranges...\x1b[0m");
            let mut regions_to_block = Vec::new();
            for reg in all_regions {
                let reg_string = reg.to_string();
                if !selected.contains(&reg_string) {
                    regions_to_block.push(reg_string);
                }
            }
            
            let (v4, v6) = match fetch_aws_ip_ranges(&regions_to_block) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("\x1b[91mFailed to fetch AWS IPs:\x1b[0m {}", e);
                    return;
                }
            };
            
            script.push_str("ipset restore <<-EOF\n");
            script.push_str("create dbdqueue_block hash:net -exist\n");
            script.push_str("flush dbdqueue_block\n");
            for ip in v4 {
                script.push_str(&format!("add dbdqueue_block {}\n", ip));
            }
            script.push_str("create dbdqueue_block_v6 hash:net family inet6 -exist\n");
            script.push_str("flush dbdqueue_block_v6\n");
            for ip in v6 {
                script.push_str(&format!("add dbdqueue_block_v6 {}\n", ip));
            }
            script.push_str("EOF\n\n");
            
            // Apply new rules restricting ALL traffic to block game traffic/pings over TCP as well
            script.push_str("iptables -I OUTPUT -m set --match-set dbdqueue_block dst -j REJECT\n");
            script.push_str("ip6tables -I OUTPUT -m set --match-set dbdqueue_block_v6 dst -j REJECT\n");
            
            println!("\x1b[93mRequesting elevated privileges to apply firewall rules (iptables/ipset)...\x1b[0m");
        }
        _ => {
            script.push_str("ipset destroy dbdqueue_block 2>/dev/null\n");
            script.push_str("ipset destroy dbdqueue_block_v6 2>/dev/null\n");
            println!("\x1b[93mRequesting elevated privileges to clear firewall rules...\x1b[0m");
        }
    }

    let script_path = "/tmp/dbdqueue_firewall.sh";
    if let Err(e) = std::fs::write(script_path, script) {
        eprintln!("\x1b[91mFailed to write temporary script:\x1b[0m {}", e);
        return;
    }

    let mut child = match Command::new("pkexec")
        .args(["bash", script_path])
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            eprintln!("\x1b[91mError executing pkexec:\x1b[0m {}", e);
            return;
        }
    };

    match child.wait() {
        Ok(status) => {
            if status.success() {
                println!("\x1b[92mSuccessfully updated Region Lock firewall rules!\x1b[0m");
            } else {
                eprintln!("\x1b[91mFailed to apply firewall rules (code {:?})\x1b[0m", status.code());
            }
        }
        Err(e) => {
            eprintln!("\x1b[91mError waiting for pkexec:\x1b[0m {}", e);
        }
    }
}

#[cfg(windows)]
pub fn update_firewall(_selected_aws_regions: Option<&[String]>) {
    eprintln!("\x1b[91mWindows firewall region lock is not yet implemented in this version.\x1b[0m");
}
