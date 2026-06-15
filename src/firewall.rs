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

#[derive(Deserialize)]
struct SteamSdrConfig {
    pops: std::collections::HashMap<String, SteamPop>,
}

#[derive(Deserialize)]
struct SteamPop {
    relays: Option<Vec<SteamRelay>>,
}

#[derive(Deserialize)]
struct SteamRelay {
    ipv4: String,
}

fn get_aws_to_sdr_map() -> std::collections::HashMap<&'static str, Vec<&'static str>> {
    let mut m = std::collections::HashMap::new();
    m.insert("us-east-1", vec!["iad", "atl"]);
    m.insert("us-east-2", vec!["ord", "dfw"]);
    m.insert("us-west-1", vec!["lax"]);
    m.insert("us-west-2", vec!["sea", "eat"]);
    m.insert("ca-central-1", vec!["ord", "iad"]);
    m.insert("eu-central-1", vec!["fra", "fsn", "vie", "waw"]);
    m.insert("eu-west-1", vec!["ams", "hel", "sto", "sto2"]);
    m.insert("eu-west-2", vec!["lhr", "par", "mad"]);
    m.insert("sa-east-1", vec!["gru", "eze", "lim", "scl"]);
    m.insert("ap-south-1", vec!["bom2", "maa2", "dxb"]);
    m.insert("ap-east-1", vec!["hkg"]);
    m.insert("ap-northeast-1", vec!["tyo"]);
    m.insert("ap-northeast-2", vec!["seo"]);
    m.insert("ap-southeast-1", vec!["sgp", "jnb"]);
    m.insert("ap-southeast-2", vec!["syd"]);
    m
}

fn fetch_steam_sdr_ranges(allowed_pops: &[String]) -> Result<Vec<String>, String> {
    let url = "https://api.steampowered.com/ISteamApps/GetSDRConfig/v1/?appid=381210";
    let resp = ureq::get(url)
        .set("User-Agent", "curl/8.7.1")
        .call()
        .map_err(|e| e.to_string())?;

    let body = resp.into_string().map_err(|e| e.to_string())?;
    let sdr_data: SteamSdrConfig = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    
    let mut blocked_ips = Vec::new();
    for (pop_code, pop) in sdr_data.pops {
        if !allowed_pops.contains(&pop_code) {
            if let Some(relays) = pop.relays {
                for relay in relays {
                    if let Some(pos) = relay.ipv4.rfind('.') {
                        let subnet = format!("{}.0/24", &relay.ipv4[..pos]);
                        blocked_ips.push(subnet);
                    }
                }
            }
        }
    }
    
    blocked_ips.sort();
    blocked_ips.dedup();
    
    Ok(blocked_ips)
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
                // Always allow us-east-1 (Virginia) AWS IPs because they host the master game/login APIs.
                // Matchmaking to Virginia is still successfully blocked via its Steam SDR relays (iad/atl).
                if reg_string == "us-east-1" {
                    continue;
                }
                if !selected.contains(&reg_string) {
                    regions_to_block.push(reg_string);
                }
            }
            
            let (mut v4, v6) = match fetch_aws_ip_ranges(&regions_to_block) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("\x1b[91mFailed to fetch AWS IPs:\x1b[0m {}", e);
                    return;
                }
            };
            
            // Collect allowed POP codes
            let aws_to_sdr = get_aws_to_sdr_map();
            let mut allowed_pops = Vec::new();
            for reg in selected {
                if let Some(sdr_pops) = aws_to_sdr.get(reg.as_str()) {
                    for pop in sdr_pops {
                        allowed_pops.push(pop.to_string());
                    }
                }
            }
            
            println!("\x1b[94mFetching Steam SDR configuration...\x1b[0m");
            let sdr_subnets = match fetch_steam_sdr_ranges(&allowed_pops) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("\x1b[91mFailed to fetch Steam SDR config:\x1b[0m {}", e);
                    return;
                }
            };
            
            // Append Steam SDR subnets to v4 list so they are blocked together
            v4.extend(sdr_subnets);
            v4.sort();
            v4.dedup();
            
            let v4_elements = v4.join(",\n            ");
            let v6_elements = v6.join(",\n            ");
            
            script.push_str("if command -v nft &>/dev/null; then\n");
            script.push_str("    nft delete table inet dbdqueue 2>/dev/null\n");
            script.push_str("    cat << 'EOF' > /tmp/dbdqueue_nft.conf\n");
            script.push_str("table inet dbdqueue {\n");
            script.push_str("    set blocked_ips {\n");
            script.push_str("        type ipv4_addr\n");
            script.push_str("        flags interval\n");
            script.push_str("        auto-merge\n");
            if !v4_elements.is_empty() {
                script.push_str("        elements = {\n");
                script.push_str(&format!("            {}\n", v4_elements));
                script.push_str("        }\n");
            }
            script.push_str("    }\n");
            script.push_str("    set blocked_ips_v6 {\n");
            script.push_str("        type ipv6_addr\n");
            script.push_str("        flags interval\n");
            script.push_str("        auto-merge\n");
            if !v6_elements.is_empty() {
                script.push_str("        elements = {\n");
                script.push_str(&format!("            {}\n", v6_elements));
                script.push_str("        }\n");
            }
            script.push_str("    }\n");
            script.push_str("    chain output {\n");
            script.push_str("        type filter hook output priority filter - 10; policy accept;\n");
            script.push_str("        ip daddr @blocked_ips reject\n");
            script.push_str("        ip6 daddr @blocked_ips_v6 reject\n");
            script.push_str("    }\n");
            script.push_str("}\n");
            script.push_str("EOF\n");
            script.push_str("    nft -f /tmp/dbdqueue_nft.conf\n");
            script.push_str("    rm -f /tmp/dbdqueue_nft.conf\n");
            script.push_str("else\n");
            
            script.push_str("    ipset restore <<'EOF'\n");
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
            script.push_str("EOF\n");
            
            // Apply new rules restricting ALL traffic to block game traffic/pings over TCP as well
            script.push_str("    iptables -I OUTPUT -m set --match-set dbdqueue_block dst -j REJECT\n");
            script.push_str("    ip6tables -I OUTPUT -m set --match-set dbdqueue_block_v6 dst -j REJECT\n");
            script.push_str("fi\n");
            
            println!("\x1b[93mRequesting elevated privileges to apply firewall rules...\x1b[0m");
        }
        _ => {
            script.push_str("ipset destroy dbdqueue_block 2>/dev/null\n");
            script.push_str("ipset destroy dbdqueue_block_v6 2>/dev/null\n");
            script.push_str("if command -v nft &>/dev/null; then\n");
            script.push_str("    nft delete table inet dbdqueue 2>/dev/null\n");
            script.push_str("fi\n");
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
