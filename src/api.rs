use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

pub static API_TO_AWS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
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
});

pub fn get_api_to_aws() -> &'static HashMap<&'static str, &'static str> {
    &API_TO_AWS
}

pub fn get_all_aws_regions() -> Vec<&'static str> {
    vec![
        "eu-central-1",
        "eu-west-1",
        "eu-west-2",
        "us-east-1",
        "us-east-2",
        "us-west-1",
        "us-west-2",
        "ca-central-1",
        "sa-east-1",
        "ap-south-1",
        "ap-east-1",
        "ap-northeast-1",
        "ap-northeast-2",
        "ap-southeast-1",
        "ap-southeast-2",
    ]
}

pub static AWS_TO_API: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    for (&k, &v) in API_TO_AWS.iter() {
        m.insert(v, k);
    }
    m
});

pub fn get_aws_to_api() -> &'static HashMap<&'static str, &'static str> {
    &AWS_TO_API
}

pub static AWS_TO_FLAG: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("eu-central-1", "[DE]");
    m.insert("eu-west-1", "[IE]");
    m.insert("eu-west-2", "[GB]");
    m.insert("us-east-1", "[US]");
    m.insert("us-east-2", "[US]");
    m.insert("us-west-1", "[US]");
    m.insert("us-west-2", "[US]");
    m.insert("ca-central-1", "[CA]");
    m.insert("sa-east-1", "[BR]");
    m.insert("ap-south-1", "[IN]");
    m.insert("ap-east-1", "[HK]");
    m.insert("ap-northeast-1", "[JP]");
    m.insert("ap-northeast-2", "[KR]");
    m.insert("ap-southeast-1", "[SG]");
    m.insert("ap-southeast-2", "[AU]");
    m
});

pub fn get_aws_to_flag() -> &'static HashMap<&'static str, &'static str> {
    &AWS_TO_FLAG
}

pub fn get_disabled_aws_regions(queues: &[RegionQueueData]) -> std::collections::HashSet<String> {
    let mut disabled = std::collections::HashSet::new();
    for q in queues {
        if q.mode == "Standard"
            && q.is_disabled()
            && let Some(code) = API_TO_AWS.get(q.name.as_str())
        {
            disabled.insert(code.to_string());
        }
    }
    disabled
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionQueueData {
    pub flag: String,
    pub name: String,
    pub mode: String, // "Standard" or "Event"
    pub survivor: String,
    pub killer: String,
}

impl RegionQueueData {
    pub fn is_disabled(&self) -> bool {
        self.survivor == "—" && self.killer == "—"
    }
}

pub fn parse_time_to_seconds(time_str: &str) -> u32 {
    let s = time_str.trim();
    if s.is_empty() || s == "—" {
        return 999999;
    }

    if let Some(pos) = s.find(|c| c == 'h' || c == 'H') {
        let num_str = s[..pos].trim();
        if let Ok(hours) = num_str.parse::<u32>() {
            return hours.saturating_mul(3600);
        }
    }

    if s.contains(':') {
        let parts: Vec<&str> = s.split(':').map(|p| p.trim()).collect();
        if parts.len() == 2 {
            if let (Ok(m), Ok(sec)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                let total = m.saturating_mul(60).saturating_add(sec);
                return if total > 0 { total } else { 999999 };
            }
        } else if parts.len() == 3 {
            if let (Ok(h), Ok(m), Ok(sec)) = (
                parts[0].parse::<u32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>(),
            ) {
                let total = h
                    .saturating_mul(3600)
                    .saturating_add(m.saturating_mul(60))
                    .saturating_add(sec);
                return if total > 0 { total } else { 999999 };
            }
        }
    }

    let mut total = 0u32;
    let mut current_num = 0u32;
    let mut has_num = false;

    for c in s.chars() {
        if let Some(digit) = c.to_digit(10) {
            current_num = current_num.saturating_mul(10).saturating_add(digit);
            has_num = true;
        } else if c == 'm' || c == 'M' {
            if has_num {
                total = total.saturating_add(current_num.saturating_mul(60));
                current_num = 0;
                has_num = false;
            }
        } else if (c == 's' || c == 'S') && has_num {
            total = total.saturating_add(current_num);
            current_num = 0;
            has_num = false;
        }
    }
    if has_num {
        total = total.saturating_add(current_num);
    }

    if total > 0 { total } else { 999999 }
}

#[derive(Deserialize, Debug)]
struct QueueTime {
    time: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct QueueData {
    killer: Option<QueueTime>,
    survivor: Option<QueueTime>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Api2Response {
    lastupdated2: i64,
    queues: HashMap<String, HashMap<String, QueueData>>,
}

pub fn format_seconds_to_time(seconds_str: &str) -> String {
    if let Ok(sec) = seconds_str.parse::<u32>() {
        if sec == 0 {
            "—".to_string()
        } else if sec < 60 {
            format!("{}s", sec)
        } else if sec < 3600 {
            let m = sec / 60;
            let s = sec % 60;
            format!("{}:{:02}", m, s)
        } else {
            let h = sec / 3600;
            format!("{}h+", h)
        }
    } else {
        "—".to_string()
    }
}

pub fn get_api_url() -> String {
    if let Ok(env_url) = std::env::var("DBD_API_URL") {
        let trimmed = env_url.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let config_path = crate::config::get_config_path();
    let config = crate::config::load_config(&config_path);
    if let Some(cfg_url) = config.api_url {
        let trimmed = cfg_url.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    "https://api2.deadbyqueue.com/queues".to_string()
}

pub fn parse_queue_response(body: &str, status: u16) -> Result<Api2Response, String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "Empty response received from API (HTTP {})",
            status
        ));
    }

    if trimmed.starts_with('<') {
        let snippet: String = trimmed.chars().take(80).collect();
        let clean_snippet = snippet.replace(['\r', '\n'], " ");
        return Err(format!(
            "Received HTML block/page instead of JSON (HTTP {}): {}...",
            status,
            clean_snippet.trim()
        ));
    }

    serde_json::from_str::<Api2Response>(trimmed).map_err(|e| {
        let snippet: String = trimmed.chars().take(80).collect();
        let clean_snippet = snippet.replace(['\r', '\n'], " ");
        format!(
            "Error parsing JSON ({}) at line {} col {}: {}...",
            e,
            e.line(),
            e.column(),
            clean_snippet.trim()
        )
    })
}

pub fn fetch_queue_times() -> Result<(Vec<RegionQueueData>, i64), String> {
    let url = get_api_url();
    let agent = ureq::builder()
        .timeout(std::time::Duration::from_secs(10))
        .try_proxy_from_env(true)
        .build();

    let resp = agent
        .get(&url)
        .set("User-Agent", "curl/8.7.1")
        .set("Accept", "application/json, text/plain, */*")
        .call()
        .map_err(|e| format!("Error connecting to API: {}", e))?;

    let status = resp.status();
    let body = resp
        .into_string()
        .map_err(|e| format!("Error reading response (HTTP {}): {}", status, e))?;

    let api_data = parse_queue_response(&body, status)?;

    let aws_to_api = get_aws_to_api();
    let aws_to_flag = get_aws_to_flag();
    let all_regions = get_all_aws_regions();

    let mut data = Vec::new();

    for mode_name in &["Standard", "Event"] {
        let json_mode_key = if *mode_name == "Standard" {
            "live"
        } else {
            "live-event"
        };
        if let Some(mode_queues) = api_data.queues.get(json_mode_key) {
            for reg in &all_regions {
                let name = aws_to_api.get(reg).unwrap_or(reg).to_string();
                let flag = aws_to_flag.get(reg).unwrap_or(&"").to_string();

                let (survivor, killer) = if let Some(q_data) = mode_queues.get(*reg) {
                    let s_time = q_data
                        .survivor
                        .as_ref()
                        .map(|t| format_seconds_to_time(&t.time))
                        .unwrap_or_else(|| "—".to_string());
                    let k_time = q_data
                        .killer
                        .as_ref()
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

    Ok((data, api_data.lastupdated2))
}

pub fn resolve_region_names(words_list: &[String]) -> Vec<String> {
    let raw_str = words_list.join(" ");
    let mut parts = Vec::new();
    for p in raw_str.split(',') {
        let p_clean = p.trim();
        if !p_clean.is_empty() {
            parts.push(p_clean.to_string());
        }
    }

    let mut normalized_map = HashMap::new();
    normalized_map.insert("sao paulo", "São Paulo");
    normalized_map.insert("sao_paulo", "São Paulo");
    normalized_map.insert("saopaulo", "São Paulo");
    normalized_map.insert("hong kong", "Hong Kong");
    normalized_map.insert("hong_kong", "Hong Kong");
    normalized_map.insert("hongkong", "Hong Kong");
    normalized_map.insert("montreal", "Montréal");

    let api_to_aws = get_api_to_aws();
    let aws_to_api = get_aws_to_api();

    let mut resolved = Vec::new();

    for part in parts {
        let part_lower = part.to_lowercase();

        if let Some(norm) = normalized_map.get(part_lower.as_str()) {
            resolved.push(norm.to_string());
            continue;
        }

        if let Some(api_name) = aws_to_api.get(part_lower.as_str()) {
            resolved.push(api_name.to_string());
            continue;
        }

        let mut chars = part_lower.chars();
        let part_cap = match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        };

        if api_to_aws.contains_key(part_cap.as_str()) {
            resolved.push(part_cap);
            continue;
        }

        let words: Vec<&str> = part.split_whitespace().collect();
        let mut i = 0;
        while i < words.len() {
            let word = words[i].to_lowercase();
            if i + 1 < words.len() {
                let two_words = format!("{} {}", word, words[i + 1].to_lowercase());
                if let Some(norm) = normalized_map.get(two_words.as_str()) {
                    resolved.push(norm.to_string());
                    i += 2;
                    continue;
                }
            }

            if let Some(norm) = normalized_map.get(word.as_str()) {
                resolved.push(norm.to_string());
            } else if let Some(api_name) = aws_to_api.get(word.as_str()) {
                resolved.push(api_name.to_string());
            } else {
                let mut c_chars = word.chars();
                let word_cap = match c_chars.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c_chars.as_str(),
                };
                if api_to_aws.contains_key(word_cap.as_str()) {
                    resolved.push(word_cap);
                }
            }
            i += 1;
        }
    }

    resolved
}

pub fn resolve_to_aws_codes(words_list: &[String]) -> Vec<String> {
    let mut resolved = Vec::new();
    let api_to_aws = get_api_to_aws();
    let all_aws = get_all_aws_regions();
    let names = resolve_region_names(words_list);
    for r in names {
        if let Some(aws_code) = api_to_aws.get(r.as_str()) {
            resolved.push(aws_code.to_string());
        } else if all_aws.contains(&r.as_str()) {
            resolved.push(r);
        }
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time() {
        assert_eq!(parse_time_to_seconds("5s"), 5);
        assert_eq!(parse_time_to_seconds("59s"), 59);
        assert_eq!(parse_time_to_seconds("1:00"), 60);
        assert_eq!(parse_time_to_seconds("3:27"), 207);
        assert_eq!(parse_time_to_seconds("34:25"), 2065);
        assert_eq!(parse_time_to_seconds("1h+"), 3600);
        assert_eq!(parse_time_to_seconds("2h+"), 7200);
        assert_eq!(parse_time_to_seconds("3m"), 180);
        assert_eq!(parse_time_to_seconds("3m27s"), 207);
        assert_eq!(parse_time_to_seconds("—"), 999999);
        assert_eq!(parse_time_to_seconds(""), 999999);
    }

    #[test]
    fn test_format_seconds() {
        assert_eq!(format_seconds_to_time("5"), "5s");
        assert_eq!(format_seconds_to_time("59"), "59s");
        assert_eq!(format_seconds_to_time("60"), "1:00");
        assert_eq!(format_seconds_to_time("180"), "3:00");
        assert_eq!(format_seconds_to_time("207"), "3:27");
        assert_eq!(format_seconds_to_time("2065"), "34:25");
        assert_eq!(format_seconds_to_time("3599"), "59:59");
        assert_eq!(format_seconds_to_time("3600"), "1h+");
        assert_eq!(format_seconds_to_time("7200"), "2h+");
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
        assert_eq!(api_data.lastupdated2, 1781286943);

        let live_queues = api_data.queues.get("live").unwrap();
        let frank_live = live_queues.get("eu-central-1").unwrap();
        assert_eq!(frank_live.killer.as_ref().unwrap().time, "207");
        assert_eq!(frank_live.survivor.as_ref().unwrap().time, "5");
    }

    #[test]
    fn test_parse_html_error() {
        let html = "<!DOCTYPE html><html><body>Access Denied / Blocked</body></html>";
        let res = parse_queue_response(html, 200);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("Received HTML block/page instead of JSON"));
        assert!(err.contains("Access Denied"));
    }

    #[test]
    fn test_parse_empty_error() {
        let empty = "   \n";
        let res = parse_queue_response(empty, 200);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("Empty response received"));
    }

    #[test]
    fn test_get_disabled_aws_regions() {
        let queues = vec![
            RegionQueueData {
                flag: "[GB]".to_string(),
                name: "London".to_string(),
                mode: "Standard".to_string(),
                survivor: "—".to_string(),
                killer: "—".to_string(),
            },
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "15s".to_string(),
                killer: "30s".to_string(),
            },
        ];
        let disabled = get_disabled_aws_regions(&queues);
        assert!(disabled.contains("eu-west-2"));
        assert!(!disabled.contains("eu-central-1"));
    }

    #[test]
    fn test_resolve_region_names() {
        let input1 = vec!["Frankfurt,Dublin".to_string()];
        let parsed1 = resolve_region_names(&input1);
        assert_eq!(parsed1, vec!["Frankfurt", "Dublin"]);

        let input2 = vec!["sao paulo, montreal, virginia".to_string()];
        let parsed2 = resolve_region_names(&input2);
        assert_eq!(parsed2, vec!["São Paulo", "Montréal", "Virginia"]);

        let input3 = vec!["us-east-1".to_string(), "eu-central-1".to_string()];
        let parsed3 = resolve_region_names(&input3);
        assert_eq!(parsed3, vec!["Virginia", "Frankfurt"]);
    }

    #[test]
    fn test_resolve_to_aws_codes() {
        let input = vec![
            "Frankfurt".to_string(),
            "sao paulo".to_string(),
            "us-east-1".to_string(),
        ];
        let codes = resolve_to_aws_codes(&input);
        assert_eq!(codes, vec!["eu-central-1", "sa-east-1", "us-east-1"]);
    }
}
