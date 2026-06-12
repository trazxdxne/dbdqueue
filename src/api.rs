use std::collections::HashMap;
use regex::Regex;
use serde::Deserialize;

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

pub fn get_aws_to_flag() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("eu-central-1", "🇩🇪");
    m.insert("eu-west-1", "🇮🇪");
    m.insert("eu-west-2", "🇬🇧");
    m.insert("us-east-1", "🇺🇸");
    m.insert("us-east-2", "🇺🇸");
    m.insert("us-west-1", "🇺🇸");
    m.insert("us-west-2", "🇺🇸");
    m.insert("ca-central-1", "🇨🇦");
    m.insert("sa-east-1", "🇧🇷");
    m.insert("ap-south-1", "🇮🇳");
    m.insert("ap-east-1", "🇭🇰");
    m.insert("ap-northeast-1", "🇯🇵");
    m.insert("ap-northeast-2", "🇰🇷");
    m.insert("ap-southeast-1", "🇸🇬");
    m.insert("ap-southeast-2", "🇦🇺");
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

#[derive(Deserialize, Debug)]
struct QueueTime {
    time: String,
}

#[derive(Deserialize, Debug)]
struct QueueData {
    killer: Option<QueueTime>,
    survivor: Option<QueueTime>,
}

#[derive(Deserialize, Debug)]
struct Api2Response {
    lastupdated: String,
    queues: HashMap<String, HashMap<String, QueueData>>,
}

pub fn format_seconds_to_time(seconds_str: &str) -> String {
    if let Ok(sec) = seconds_str.parse::<u32>() {
        if sec == 0 {
            "—".to_string()
        } else if sec < 60 {
            format!("{}s", sec)
        } else {
            let m = sec / 60;
            let s = sec % 60;
            if s > 0 {
                format!("{}m{}s", m, s)
            } else {
                format!("{}m", m)
            }
        }
    } else {
        "—".to_string()
    }
}

pub fn fetch_queue_times() -> Result<(Vec<RegionQueueData>, String), String> {
    let url = "https://api2.deadbyqueue.com/queues";
    let resp = ureq::get(url)
        .set("User-Agent", "curl/8.7.1")
        .set("Accept", "*/*")
        .call()
        .map_err(|e| format!("Error fetching data: {}", e))?;
        
    let body = resp.into_string()
        .map_err(|e| format!("Error reading response: {}", e))?;
        
    let api_data: Api2Response = serde_json::from_str(&body)
        .map_err(|e| format!("Error parsing JSON: {}", e))?;
        
    let aws_to_api = get_aws_to_api();
    let aws_to_flag = get_aws_to_flag();
    let all_regions = get_all_aws_regions();
    
    let mut data = Vec::new();
    
    for mode_name in &["Standard", "Event"] {
        let json_mode_key = if *mode_name == "Standard" { "live" } else { "live-event" };
        if let Some(mode_queues) = api_data.queues.get(json_mode_key) {
            for reg in &all_regions {
                let name = aws_to_api.get(reg).unwrap_or(reg).to_string();
                let flag = aws_to_flag.get(reg).unwrap_or(&"").to_string();
                
                let (survivor, killer) = if let Some(q_data) = mode_queues.get(*reg) {
                    let s_time = q_data.survivor.as_ref()
                        .map(|t| format_seconds_to_time(&t.time))
                        .unwrap_or_else(|| "—".to_string());
                    let k_time = q_data.killer.as_ref()
                        .map(|t| format_seconds_to_time(&t.time))
                        .unwrap_or_else(|| "—".to_string());
                    (s_time, k_time)
                } else {
                    ("—".to_string(), "—".to_string())
                };
                
                data.push(RegionQueueData {
                    flag,
                    name,
                    mode: mode_name.to_string(),
                    survivor,
                    killer,
                });
            }
        }
    }
    
    Ok((data, api_data.lastupdated))
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
    fn test_format_seconds() {
        assert_eq!(format_seconds_to_time("5"), "5s");
        assert_eq!(format_seconds_to_time("180"), "3m");
        assert_eq!(format_seconds_to_time("207"), "3m27s");
        assert_eq!(format_seconds_to_time("0"), "—");
        assert_eq!(format_seconds_to_time("invalid"), "—");
    }

    #[test]
    fn test_parse_json_response() {
        let sample = r#"{
            "lastupdated": "2026-06-12 17:55:43",
            "lastupdated2": 1781286943,
            "queues": {
                "live": {
                    "eu-central-1": {
                        "killer": { "time": "207" },
                        "survivor": { "time": "5" }
                    },
                    "eu-west-1": {
                        "killer": { "time": "243" },
                        "survivor": { "time": "6" }
                    }
                },
                "live-event": {
                    "eu-central-1": {
                        "killer": { "time": "332" },
                        "survivor": { "time": "12" }
                    }
                }
            }
        }"#;
        
        let api_data: Api2Response = serde_json::from_str(sample).unwrap();
        assert_eq!(api_data.lastupdated, "2026-06-12 17:55:43");
        
        let live_queues = api_data.queues.get("live").unwrap();
        let frank_live = live_queues.get("eu-central-1").unwrap();
        assert_eq!(frank_live.killer.as_ref().unwrap().time, "207");
        assert_eq!(frank_live.survivor.as_ref().unwrap().time, "5");
    }
}
