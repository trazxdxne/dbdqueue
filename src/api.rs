use crate::config::TimeFormat;
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
    pub survivor_secs: Option<u32>,
    pub killer_secs: Option<u32>,
}

impl RegionQueueData {
    #[cfg(test)]
    pub fn new(
        flag: impl Into<String>,
        name: impl Into<String>,
        mode: impl Into<String>,
        survivor: impl Into<String>,
        killer: impl Into<String>,
    ) -> Self {
        let surv = survivor.into();
        let kill = killer.into();
        let surv_secs = if surv == "—" {
            None
        } else {
            let s = parse_time_to_seconds(&surv);
            if s < 999999 { Some(s) } else { None }
        };
        let kill_secs = if kill == "—" {
            None
        } else {
            let k = parse_time_to_seconds(&kill);
            if k < 999999 { Some(k) } else { None }
        };
        Self {
            flag: flag.into(),
            name: name.into(),
            mode: mode.into(),
            survivor: surv,
            killer: kill,
            survivor_secs: surv_secs,
            killer_secs: kill_secs,
        }
    }

    pub fn is_disabled(&self) -> bool {
        self.survivor == "—" && self.killer == "—"
    }

    pub fn reformat(&mut self, format: TimeFormat) {
        if self.survivor_secs.is_none() && self.survivor != "—" {
            let s = parse_time_to_seconds(&self.survivor);
            if s < 999999 {
                self.survivor_secs = Some(s);
            }
        }
        if self.killer_secs.is_none() && self.killer != "—" {
            let k = parse_time_to_seconds(&self.killer);
            if k < 999999 {
                self.killer_secs = Some(k);
            }
        }
        self.survivor = format_seconds_opt(self.survivor_secs, format);
        self.killer = format_seconds_opt(self.killer_secs, format);
    }
}

pub fn parse_time_to_seconds(time_str: &str) -> u32 {
    let s = time_str.trim();
    if s.is_empty() || s == "—" {
        return 999999;
    }

    if let Some(pos) = s.find(['h', 'H']) {
        let num_str = s[..pos].trim();
        if let Ok(hours) = num_str.parse::<u32>() {
            return hours.saturating_mul(3600);
        }
    }

    if s.contains(':') {
        let parts: Vec<&str> = s.split(':').map(|p| p.trim()).collect();
        if parts.len() == 2
            && let (Ok(m), Ok(sec)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>())
        {
            let total = m.saturating_mul(60).saturating_add(sec);
            return if total > 0 { total } else { 999999 };
        } else if parts.len() == 3
            && let (Ok(h), Ok(m), Ok(sec)) = (
                parts[0].parse::<u32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>(),
            )
        {
            let total = h
                .saturating_mul(3600)
                .saturating_add(m.saturating_mul(60))
                .saturating_add(sec);
            return if total > 0 { total } else { 999999 };
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

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EventItem {
    pub name: String,
    pub start: String,
    pub end: String,
}

#[derive(Deserialize, Debug)]
pub struct MiscResponse {
    #[allow(dead_code)]
    pub online: bool,
    #[serde(rename = "currentEvents")]
    pub current_events: Vec<EventItem>,
    #[serde(rename = "upcomingEvents")]
    pub upcoming_events: Vec<EventItem>,
}

pub fn format_seconds(sec: u32, format: TimeFormat) -> String {
    if sec == 0 {
        return "—".to_string();
    }
    match format {
        TimeFormat::Exact => {
            if sec < 3600 {
                format!("{}:{:02}", sec / 60, sec % 60)
            } else {
                let h = sec / 3600;
                let m = (sec % 3600) / 60;
                let s = sec % 60;
                format!("{h}:{m:02}:{s:02}")
            }
        }
        TimeFormat::Rounded => {
            if sec < 60 {
                format!("{}s", sec)
            } else if sec < 3600 {
                let mins = (sec + 30) / 60;
                if mins >= 60 {
                    "1h".to_string()
                } else {
                    format!("{}m", mins)
                }
            } else {
                let hours = (sec + 1800) / 3600;
                format!("{}h", hours.max(1))
            }
        }
    }
}

pub fn format_seconds_opt(secs: Option<u32>, format: TimeFormat) -> String {
    match secs {
        Some(s) if s > 0 => format_seconds(s, format),
        _ => "—".to_string(),
    }
}

#[cfg(test)]
pub fn format_seconds_to_time(seconds_str: &str, format: TimeFormat) -> String {
    if let Ok(sec) = seconds_str.parse::<u32>() {
        format_seconds(sec, format)
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

                let (survivor, killer, survivor_secs, killer_secs) =
                    if let Some(q_data) = mode_queues.get(*reg) {
                        let s_sec = q_data
                            .survivor
                            .as_ref()
                            .and_then(|t| t.time.parse::<u32>().ok());
                        let k_sec = q_data
                            .killer
                            .as_ref()
                            .and_then(|t| t.time.parse::<u32>().ok());
                        let s_time = format_seconds_opt(s_sec, TimeFormat::Exact);
                        let k_time = format_seconds_opt(k_sec, TimeFormat::Exact);
                        (s_time, k_time, s_sec, k_sec)
                    } else {
                        ("—".to_string(), "—".to_string(), None, None)
                    };

                data.push(RegionQueueData {
                    flag,
                    name,
                    mode: mode_name.to_string(),
                    survivor,
                    killer,
                    survivor_secs,
                    killer_secs,
                });
            }
        }
    }

    Ok((data, api_data.lastupdated2))
}

pub fn fetch_misc_data() -> Result<MiscResponse, String> {
    let agent = ureq::builder()
        .timeout(std::time::Duration::from_secs(10))
        .try_proxy_from_env(true)
        .build();

    let resp = agent
        .get("https://api2.deadbyqueue.com/misc")
        .set("User-Agent", "curl/8.7.1")
        .set("Accept", "application/json")
        .call()
        .map_err(|e| format!("Error connecting to misc API: {}", e))?;

    let status = resp.status();
    let body = resp
        .into_string()
        .map_err(|e| format!("Error reading misc response (HTTP {}): {}", status, e))?;

    serde_json::from_str::<MiscResponse>(body.trim())
        .map_err(|e| format!("Error parsing misc JSON: {}", e))
}

pub fn format_event_countdown(remaining_secs: u64, format: TimeFormat) -> String {
    let days = remaining_secs / 86400;
    let hours = (remaining_secs % 86400) / 3600;
    let mins = (remaining_secs % 3600) / 60;
    let secs = remaining_secs % 60;

    match format {
        TimeFormat::Exact => {
            if days >= 1 {
                format!("{}d {:02}:{:02}:{:02}", days, hours, mins, secs)
            } else if remaining_secs >= 3600 {
                format!("{:02}:{:02}:{:02}", hours, mins, secs)
            } else if remaining_secs >= 60 {
                format!("{:02}:{:02}", mins, secs)
            } else {
                format!("00:{:02}", secs)
            }
        }
        TimeFormat::Rounded => {
            if days >= 1 {
                format!("{}d", days)
            } else if remaining_secs >= 3600 {
                format!("{}h", hours)
            } else if remaining_secs >= 60 {
                format!("{}m", mins)
            } else {
                format!("{}s", secs)
            }
        }
    }
}

pub fn event_timestamp_to_local(ts_str: &str) -> Option<chrono::DateTime<chrono::Local>> {
    let ts: i64 = ts_str.trim().parse().ok()?;
    let utc = chrono::DateTime::from_timestamp(ts, 0)?;
    Some(utc.with_timezone(&chrono::Local))
}

pub fn format_local_datetime(dt: chrono::DateTime<chrono::Local>) -> String {
    dt.format("%b %d, %H:%M").to_string()
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
        // Exact format
        assert_eq!(parse_time_to_seconds("0:01"), 1);
        assert_eq!(parse_time_to_seconds("0:53"), 53);
        assert_eq!(parse_time_to_seconds("1:00"), 60);
        assert_eq!(parse_time_to_seconds("3:27"), 207);
        assert_eq!(parse_time_to_seconds("4:54"), 294);
        assert_eq!(parse_time_to_seconds("30:52"), 1852);
        assert_eq!(parse_time_to_seconds("34:25"), 2065);
        assert_eq!(parse_time_to_seconds("1:00:00"), 3600);
        assert_eq!(parse_time_to_seconds("1:52:31"), 6751);

        // Rounded format
        assert_eq!(parse_time_to_seconds("1s"), 1);
        assert_eq!(parse_time_to_seconds("53s"), 53);
        assert_eq!(parse_time_to_seconds("59s"), 59);
        assert_eq!(parse_time_to_seconds("1m"), 60);
        assert_eq!(parse_time_to_seconds("5m"), 300);
        assert_eq!(parse_time_to_seconds("31m"), 1860);
        assert_eq!(parse_time_to_seconds("1h"), 3600);
        assert_eq!(parse_time_to_seconds("2h"), 7200);

        // Legacy formats
        assert_eq!(parse_time_to_seconds("1h+"), 3600);
        assert_eq!(parse_time_to_seconds("2h+"), 7200);
        assert_eq!(parse_time_to_seconds("3m"), 180);
        assert_eq!(parse_time_to_seconds("3m27s"), 207);

        // Disabled / empty
        assert_eq!(parse_time_to_seconds("—"), 999999);
        assert_eq!(parse_time_to_seconds(""), 999999);
    }

    #[test]
    fn test_format_seconds_exact() {
        assert_eq!(format_seconds_to_time("0", TimeFormat::Exact), "—");
        assert_eq!(format_seconds_to_time("1", TimeFormat::Exact), "0:01");
        assert_eq!(format_seconds_to_time("53", TimeFormat::Exact), "0:53");
        assert_eq!(format_seconds_to_time("60", TimeFormat::Exact), "1:00");
        assert_eq!(format_seconds_to_time("180", TimeFormat::Exact), "3:00");
        assert_eq!(format_seconds_to_time("207", TimeFormat::Exact), "3:27");
        assert_eq!(format_seconds_to_time("294", TimeFormat::Exact), "4:54");
        assert_eq!(format_seconds_to_time("1852", TimeFormat::Exact), "30:52");
        assert_eq!(format_seconds_to_time("2065", TimeFormat::Exact), "34:25");
        assert_eq!(format_seconds_to_time("3599", TimeFormat::Exact), "59:59");
        assert_eq!(format_seconds_to_time("3600", TimeFormat::Exact), "1:00:00");
        assert_eq!(format_seconds_to_time("6751", TimeFormat::Exact), "1:52:31");
        assert_eq!(format_seconds_to_time("invalid", TimeFormat::Exact), "—");
    }

    #[test]
    fn test_format_seconds_rounded() {
        assert_eq!(format_seconds_to_time("0", TimeFormat::Rounded), "—");
        assert_eq!(format_seconds_to_time("1", TimeFormat::Rounded), "1s");
        assert_eq!(format_seconds_to_time("53", TimeFormat::Rounded), "53s");
        assert_eq!(format_seconds_to_time("59", TimeFormat::Rounded), "59s");
        assert_eq!(format_seconds_to_time("60", TimeFormat::Rounded), "1m");
        assert_eq!(format_seconds_to_time("89", TimeFormat::Rounded), "1m");
        assert_eq!(format_seconds_to_time("90", TimeFormat::Rounded), "2m");
        assert_eq!(format_seconds_to_time("294", TimeFormat::Rounded), "5m");
        assert_eq!(format_seconds_to_time("1852", TimeFormat::Rounded), "31m");
        assert_eq!(format_seconds_to_time("3569", TimeFormat::Rounded), "59m");
        assert_eq!(format_seconds_to_time("3570", TimeFormat::Rounded), "1h");
        assert_eq!(format_seconds_to_time("3599", TimeFormat::Rounded), "1h");
        assert_eq!(format_seconds_to_time("3600", TimeFormat::Rounded), "1h");
        assert_eq!(format_seconds_to_time("5399", TimeFormat::Rounded), "1h");
        assert_eq!(format_seconds_to_time("5400", TimeFormat::Rounded), "2h");
        assert_eq!(format_seconds_to_time("6751", TimeFormat::Rounded), "2h");
        assert_eq!(format_seconds_to_time("invalid", TimeFormat::Rounded), "—");
    }

    #[test]
    fn test_region_queue_data_reformat() {
        let mut row = RegionQueueData::new("[DE]", "Frankfurt", "Standard", "294", "1852");
        assert_eq!(row.survivor_secs, Some(294));
        assert_eq!(row.killer_secs, Some(1852));
        assert_eq!(row.survivor, "294"); // Initial raw string passed to new
        assert_eq!(row.killer, "1852");

        row.reformat(TimeFormat::Exact);
        assert_eq!(row.survivor, "4:54");
        assert_eq!(row.killer, "30:52");

        row.reformat(TimeFormat::Rounded);
        assert_eq!(row.survivor, "5m");
        assert_eq!(row.killer, "31m");

        row.reformat(TimeFormat::Exact);
        assert_eq!(row.survivor, "4:54");
        assert_eq!(row.killer, "30:52");
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
            RegionQueueData::new("[GB]", "London", "Standard", "—", "—"),
            RegionQueueData::new("[DE]", "Frankfurt", "Standard", "15s", "30s"),
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

    #[test]
    fn test_format_event_countdown_exact() {
        // >= 1 day: "Xd HH:MM:SS"
        assert_eq!(
            format_event_countdown(90000, TimeFormat::Exact),
            "1d 01:00:00"
        );
        assert_eq!(
            format_event_countdown(86400, TimeFormat::Exact),
            "1d 00:00:00"
        );
        assert_eq!(
            format_event_countdown(172800 + 3661, TimeFormat::Exact),
            "2d 01:01:01"
        );
        // >= 1 hour, < 1 day: "HH:MM:SS"
        assert_eq!(format_event_countdown(3661, TimeFormat::Exact), "01:01:01");
        assert_eq!(format_event_countdown(3600, TimeFormat::Exact), "01:00:00");
        // >= 1 min, < 1 hour: "MM:SS"
        assert_eq!(format_event_countdown(110, TimeFormat::Exact), "01:50");
        assert_eq!(format_event_countdown(60, TimeFormat::Exact), "01:00");
        // < 1 min: "00:SS"
        assert_eq!(format_event_countdown(45, TimeFormat::Exact), "00:45");
        assert_eq!(format_event_countdown(0, TimeFormat::Exact), "00:00");
    }

    #[test]
    fn test_format_event_countdown_rounded() {
        // >= 1 day: "Xd"
        assert_eq!(format_event_countdown(90000, TimeFormat::Rounded), "1d");
        assert_eq!(format_event_countdown(172800, TimeFormat::Rounded), "2d");
        // >= 1 hour, < 1 day: "Xh"
        assert_eq!(format_event_countdown(7200, TimeFormat::Rounded), "2h");
        assert_eq!(format_event_countdown(3600, TimeFormat::Rounded), "1h");
        // >= 1 min, < 1 hour: "Xm"
        assert_eq!(format_event_countdown(120, TimeFormat::Rounded), "2m");
        assert_eq!(format_event_countdown(60, TimeFormat::Rounded), "1m");
        // < 1 min: "Xs"
        assert_eq!(format_event_countdown(45, TimeFormat::Rounded), "45s");
        assert_eq!(format_event_countdown(0, TimeFormat::Rounded), "0s");
    }

    #[test]
    fn test_misc_response_deserialization() {
        let json = r#"{
            "online": true,
            "currentEvents": [
                {"name": "2v8 event", "start": "1788879600", "end": "1790694000"}
            ],
            "upcomingEvents": []
        }"#;
        let resp: MiscResponse = serde_json::from_str(json).unwrap();
        assert!(resp.online);
        assert_eq!(resp.current_events.len(), 1);
        assert_eq!(resp.current_events[0].name, "2v8 event");
        assert_eq!(resp.current_events[0].start, "1788879600");
        assert_eq!(resp.current_events[0].end, "1790694000");
        assert!(resp.upcoming_events.is_empty());
    }

    #[test]
    fn test_misc_response_empty_events() {
        let json = r#"{"online": false, "currentEvents": [], "upcomingEvents": []}"#;
        let resp: MiscResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.online);
        assert!(resp.current_events.is_empty());
        assert!(resp.upcoming_events.is_empty());
    }

    #[test]
    fn test_event_timestamp_to_local() {
        // Valid timestamp
        let dt = event_timestamp_to_local("1788879600");
        assert!(dt.is_some());
        // Invalid
        assert!(event_timestamp_to_local("not_a_number").is_none());
        assert!(event_timestamp_to_local("").is_none());
    }

    #[test]
    fn test_format_local_datetime() {
        let dt = event_timestamp_to_local("1788879600").unwrap();
        let s = format_local_datetime(dt);
        // Should contain month abbreviation, day, and HH:MM
        assert!(s.contains(':'), "Should contain time separator: {}", s);
        assert!(s.contains(','), "Should contain comma separator: {}", s);
    }
}
