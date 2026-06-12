use std::collections::HashMap;
use regex::Regex;

pub const GROUP1: &str = "eu-central-1,eu-west-1,eu-west-2,us-east-1,us-east-2,us-west-1,us-west-2,ca-central-1,sa-east-1";
pub const GROUP2: &str = "ap-south-1,ap-east-1,ap-northeast-1,ap-northeast-2,ap-southeast-1,ap-southeast-2";

pub fn get_api_to_aws() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("Frankfurt", "eu-central-1");
    m.insert("Dublin", "eu-west-1");
    m.insert("London", "eu-west-2");
    m.insert("Virginia", "us-east-1");
    m.insert("Ohio", "us-east-2");
    m.insert("California", "us-west-1");
    m.insert("Oregon", "us-west-2");
    m.insert("Montréal", "ca-central-1");
    m.insert("São Paulo", "sa-east-1");
    m.insert("Mumbai", "ap-south-1");
    m.insert("Hong Kong", "ap-east-1");
    m.insert("Tokyo", "ap-northeast-1");
    m.insert("Seoul", "ap-northeast-2");
    m.insert("Singapore", "ap-southeast-1");
    m.insert("Sydney", "ap-southeast-2");
    m
}

pub fn get_all_aws_regions() -> Vec<&'static str> {
    vec![
        "eu-central-1", "eu-west-1", "eu-west-2",
        "us-east-1", "us-east-2", "us-west-1",
        "us-west-2", "ca-central-1", "sa-east-1",
        "ap-south-1", "ap-east-1", "ap-northeast-1",
        "ap-northeast-2", "ap-southeast-1", "ap-southeast-2"
    ]
}

pub fn get_aws_to_api() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    for (k, v) in get_api_to_aws() {
        m.insert(v, k);
    }
    m
}

#[derive(Debug, Clone)]
pub struct RegionQueueData {
    pub flag: String,
    pub name: String,
    pub mode: String, // "Standard" or "Event"
    pub survivor: String,
    pub killer: String,
}

pub fn clean_region_name(raw_name: &str) -> String {
    let re = Regex::new(r"[^\w\s]+").unwrap();
    let cleaned = re.replace_all(raw_name, "");
    cleaned.replace("Event", "").trim().to_string()
}

pub fn parse_time_to_seconds(time_str: &str) -> u32 {
    if time_str == "—" || time_str.is_empty() {
        return 999999;
    }
    
    let re = Regex::new(r"(?:(\d+)m)?(?:(\d+)s)?").unwrap();
    if let Some(caps) = re.captures(time_str) {
        let mut total = 0;
        if let Some(m) = caps.get(1)
            && let Ok(min) = m.as_str().parse::<u32>() {
                total += min * 60;
            }
        if let Some(s) = caps.get(2)
            && let Ok(sec) = s.as_str().parse::<u32>() {
                total += sec;
            }
        if total > 0 {
            return total;
        }
    }
    999999
}

pub fn fetch_queue_times() -> Result<Vec<RegionQueueData>, String> {
    let mut responses = Vec::new();
    
    for group in &[GROUP1, GROUP2] {
        let url = format!(
            "https://api.deadbyqueue.com/queuetime?region={}&mode=live,live-event&extras=flag,regionname",
            group
        );
        let resp = ureq::get(&url)
            .set("User-Agent", "curl/8.7.1")
            .set("Accept", "*/*")
            .call()
            .map_err(|e| format!("Error fetching data: {}", e))?;
            
        let body = resp.into_string()
            .map_err(|e| format!("Error reading response: {}", e))?;
        responses.push(body);
    }
    
    let combined = responses.join(" | ");
    Ok(parse_api_response(&combined))
}

pub fn parse_api_response(text: &str) -> Vec<RegionQueueData> {
    let mut data = Vec::new();
    let re_main = Regex::new(
        r"^(.*?)\s+([^\s]+)\s*/\s*([^\s,]+)(?:,\s*Event:\s*([^\s]+)\s*/\s*([^\s]+))?$"
    ).unwrap();
    let re_emoji = Regex::new(r"^([^\w\s]+)\s*(.*)$").unwrap();
    
    for part in text.split('|') {
        let part_trimmed = part.trim();
        if part_trimmed.is_empty() {
            continue;
        }
        
        if let Some(caps) = re_main.captures(part_trimmed) {
            let flag_and_name = caps.get(1).map_or("", |m| m.as_str()).trim();
            let k_std = caps.get(2).map_or("—", |m| m.as_str()).to_string();
            let s_std = caps.get(3).map_or("—", |m| m.as_str()).to_string();
            let k_ev = caps.get(4).map(|m| m.as_str().to_string());
            let s_ev = caps.get(5).map(|m| m.as_str().to_string());
            
            let (flag, name) = if let Some(em_caps) = re_emoji.captures(flag_and_name) {
                (
                    em_caps.get(1).map_or("", |m| m.as_str()).to_string(),
                    em_caps.get(2).map_or("", |m| m.as_str()).to_string(),
                )
            } else {
                ("".to_string(), flag_and_name.to_string())
            };
            
            let name_clean = clean_region_name(&name);
            
            data.push(RegionQueueData {
                flag: flag.clone(),
                name: name_clean.clone(),
                mode: "Standard".to_string(),
                survivor: s_std,
                killer: k_std,
            });
            
            if let (Some(ke), Some(se)) = (k_ev, s_ev) {
                data.push(RegionQueueData {
                    flag,
                    name: name_clean,
                    mode: "Event".to_string(),
                    survivor: se,
                    killer: ke,
                });
            }
        } else {
            // Handle offline or differently formatted parts
            let (flag, name) = if let Some(em_caps) = re_emoji.captures(part_trimmed) {
                (
                    em_caps.get(1).map_or("", |m| m.as_str()).to_string(),
                    em_caps.get(2).map_or("", |m| m.as_str()).to_string(),
                )
            } else {
                ("".to_string(), part_trimmed.to_string())
            };
            
            // Clean the name of offline flags
            let name_clean = name
                .replace("❌", "")
                .replace("Offline", "")
                .replace(",", "")
                .replace("Event:", "");
            let name_clean = clean_region_name(&name_clean);
            
            data.push(RegionQueueData {
                flag: flag.clone(),
                name: name_clean.clone(),
                mode: "Standard".to_string(),
                survivor: "—".to_string(),
                killer: "—".to_string(),
            });
            
            data.push(RegionQueueData {
                flag,
                name: name_clean,
                mode: "Event".to_string(),
                survivor: "—".to_string(),
                killer: "—".to_string(),
            });
        }
    }
    
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time() {
        assert_eq!(parse_time_to_seconds("5s"), 5);
        assert_eq!(parse_time_to_seconds("3m"), 180);
        assert_eq!(parse_time_to_seconds("3m27s"), 207);
        assert_eq!(parse_time_to_seconds("—"), 999999);
        assert_eq!(parse_time_to_seconds(""), 999999);
    }

    #[test]
    fn test_clean_name() {
        assert_eq!(clean_region_name("🇩🇪 Frankfurt"), "Frankfurt");
        assert_eq!(clean_region_name(" São Paulo  "), "São Paulo");
        assert_eq!(clean_region_name("Montréal Event"), "Montréal");
    }

    #[test]
    fn test_parse_response() {
        let sample = "🇩🇪 Frankfurt 3m27s / 5s, Event: 5m32s / 12s | 🇮🇪 Dublin 4m3s / 6s | ❌ London Offline";
        let parsed = parse_api_response(sample);
        
        assert!(parsed.len() >= 4);
        
        let frank_std = parsed.iter().find(|r| r.name == "Frankfurt" && r.mode == "Standard").unwrap();
        assert_eq!(frank_std.killer, "3m27s");
        assert_eq!(frank_std.survivor, "5s");
        
        let frank_ev = parsed.iter().find(|r| r.name == "Frankfurt" && r.mode == "Event").unwrap();
        assert_eq!(frank_ev.killer, "5m32s");
        assert_eq!(frank_ev.survivor, "12s");

        let dub_std = parsed.iter().find(|r| r.name == "Dublin" && r.mode == "Standard").unwrap();
        assert_eq!(dub_std.killer, "4m3s");
        assert_eq!(dub_std.survivor, "6s");
        
        let lon_std = parsed.iter().find(|r| r.name == "London" && r.mode == "Standard").unwrap();
        assert_eq!(lon_std.killer, "—");
        assert_eq!(lon_std.survivor, "—");
    }
}

