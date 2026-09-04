use crate::api::{self, RegionQueueData};
use crate::config::{AppConfig, GameMode, Language, SortOrder};
use crate::i18n::{self, Locale};
use ratatui::widgets::TableState;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

pub fn normalize_key_char(c: char) -> char {
    match c {
        'й' | 'Й' => 'q',
        'ц' | 'Ц' => 'w',
        'у' | 'У' => 'e',
        'к' | 'К' => 'r',
        'е' | 'Е' => 't',
        'н' | 'Н' => 'y',
        'г' | 'Г' => 'u',
        'ш' | 'Ш' => 'i',
        'щ' | 'Щ' => 'o',
        'з' | 'З' => 'p',
        'ф' | 'Ф' => 'a',
        'ы' | 'Ы' => 's',
        'в' | 'В' => 'd',
        'а' | 'А' => 'f',
        'п' | 'П' => 'g',
        'р' | 'Р' => 'h',
        'о' | 'О' => 'j',
        'л' | 'Л' => 'k',
        'д' | 'Д' => 'l',
        'я' | 'Я' => 'z',
        'ч' | 'Ч' => 'x',
        'с' | 'С' => 'c',
        'м' | 'М' => 'v',
        'и' | 'И' => 'b',
        'т' | 'Т' => 'n',
        'ь' | 'Ь' => 'm',
        other => other.to_ascii_lowercase(),
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AppAction {
    None,
    Refresh,
    SaveConfig(AppConfig),
    ApplyLocks(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Error,
    Info,
    Success,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub message: String,
    pub kind: NoticeKind,
    pub expires_at: Option<Instant>,
}

impl Notice {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: NoticeKind::Error,
            expires_at: None,
        }
    }

    pub fn info(message: impl Into<String>, now: Instant, duration: std::time::Duration) -> Self {
        Self {
            message: message.into(),
            kind: NoticeKind::Info,
            expires_at: Some(now + duration),
        }
    }

    pub fn success(
        message: impl Into<String>,
        now: Instant,
        duration: std::time::Duration,
    ) -> Self {
        Self {
            message: message.into(),
            kind: NoticeKind::Success,
            expires_at: Some(now + duration),
        }
    }
}

pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

pub const NEAR_BEST_TOLERANCE_SECS: u32 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BestPick<'a> {
    pub row: &'a RegionQueueData,
    pub similar: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary<'a> {
    pub killer: Option<BestPick<'a>>,
    pub survivor: Option<BestPick<'a>>,
    pub lowest_ping: Option<&'a RegionQueueData>,
}

pub struct App {
    pub queues: Vec<RegionQueueData>,
    pub api_last_updated: i64,
    pub pings: HashMap<String, u32>,
    pub sort: SortOrder,
    pub mode: GameMode,
    pub priority: Vec<String>,
    pub locked: HashSet<String>,
    pub notice: Option<Notice>,
    pub should_quit: bool,
    pub is_fetching: bool,
    pub table_state: TableState,
    pub show_lock_modal: bool,
    pub lock_modal_selected: Vec<String>,
    pub lock_modal_cursor: usize,
    pub spinner_frame: usize,
    pub locale: Locale,
    pub lang: Language,
    pub api_url: Option<String>,
}

impl App {
    pub fn new(
        sort: SortOrder,
        mode: GameMode,
        priority: Vec<String>,
        locked: Vec<String>,
        lang: Language,
        api_url: Option<String>,
    ) -> Self {
        let locale = i18n::resolve_locale(lang);
        Self {
            queues: Vec::new(),
            api_last_updated: 0,
            pings: HashMap::new(),
            sort,
            mode,
            priority,
            locked: locked.into_iter().collect(),
            notice: None,
            should_quit: false,
            is_fetching: true,
            table_state: TableState::default(),
            show_lock_modal: false,
            lock_modal_selected: Vec::new(),
            lock_modal_cursor: 0,
            spinner_frame: 0,
            locale,
            lang,
            api_url,
        }
    }

    pub fn to_config(&self) -> AppConfig {
        let mut locked_vec: Vec<String> = self.locked.iter().cloned().collect();
        locked_vec.sort();
        AppConfig {
            priority: self.priority.clone(),
            locked: locked_vec,
            sort: self.sort,
            mode: self.mode,
            lang: self.lang,
            api_url: self.api_url.clone(),
        }
    }

    pub fn move_selection(&mut self, direction: Direction) {
        let total = self.get_filtered_sorted_rows().len();
        if total == 0 {
            self.table_state.select(None);
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => match direction {
                Direction::Down => {
                    if i >= total.saturating_sub(1) {
                        0
                    } else {
                        i + 1
                    }
                }
                Direction::Up => {
                    if i == 0 {
                        total.saturating_sub(1)
                    } else {
                        i - 1
                    }
                }
            },
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn next(&mut self) {
        self.move_selection(Direction::Down);
    }

    pub fn previous(&mut self) {
        self.move_selection(Direction::Up);
    }

    pub fn clamp_selection(&mut self) {
        let total = self.get_filtered_sorted_rows().len();
        if total == 0 {
            self.table_state.select(None);
        } else if let Some(sel) = self.table_state.selected()
            && sel >= total
        {
            self.table_state.select(Some(total - 1));
        }

        let modal_total = self.get_modal_regions().len();
        if modal_total == 0 {
            self.lock_modal_cursor = 0;
        } else if self.lock_modal_cursor >= modal_total {
            self.lock_modal_cursor = modal_total - 1;
        }
    }

    pub fn get_modal_regions(&self) -> Vec<&'static str> {
        let disabled = api::get_disabled_aws_regions(&self.queues);
        let mut regions: Vec<&'static str> = api::get_all_aws_regions()
            .into_iter()
            .filter(|reg| !disabled.contains(*reg))
            .collect();
        let aws_to_api = api::get_aws_to_api();
        regions.sort_by_cached_key(|&reg| {
            let ping = self.pings.get(reg).copied().unwrap_or(u32::MAX);
            let name = aws_to_api.get(reg).unwrap_or(&reg);
            (ping, *name)
        });
        regions
    }

    pub fn open_lock_modal(&mut self) {
        self.show_lock_modal = true;
        let mut sel: Vec<String> = self.locked.iter().cloned().collect();
        sel.sort();
        self.lock_modal_selected = sel;
        self.lock_modal_cursor = 0;
        self.clamp_selection();
    }

    pub fn move_modal_cursor(&mut self, direction: Direction) {
        let regions = self.get_modal_regions();
        if regions.is_empty() {
            self.lock_modal_cursor = 0;
            return;
        }
        match direction {
            Direction::Up => {
                if self.lock_modal_cursor == 0 {
                    self.lock_modal_cursor = regions.len().saturating_sub(1);
                } else {
                    self.lock_modal_cursor -= 1;
                }
            }
            Direction::Down => {
                if self.lock_modal_cursor >= regions.len().saturating_sub(1) {
                    self.lock_modal_cursor = 0;
                } else {
                    self.lock_modal_cursor += 1;
                }
            }
        }
    }

    pub fn lock_modal_up(&mut self) {
        self.move_modal_cursor(Direction::Up);
    }

    pub fn lock_modal_down(&mut self) {
        self.move_modal_cursor(Direction::Down);
    }

    pub fn lock_modal_toggle(&mut self) {
        let regions = self.get_modal_regions();
        if let Some(code) = regions.get(self.lock_modal_cursor) {
            let code_str = code.to_string();
            if let Some(pos) = self.lock_modal_selected.iter().position(|r| r == &code_str) {
                self.lock_modal_selected.remove(pos);
            } else {
                self.lock_modal_selected.push(code_str);
            }
        }
    }

    pub fn apply_lock_modal(&mut self) -> AppAction {
        self.show_lock_modal = false;
        AppAction::ApplyLocks(self.lock_modal_selected.clone())
    }

    pub fn cancel_lock_modal(&mut self) {
        self.show_lock_modal = false;
    }

    pub fn cycle_sort(&mut self) {
        self.sort = match self.sort {
            SortOrder::Default => SortOrder::Killer,
            SortOrder::Killer => SortOrder::Survivor,
            SortOrder::Survivor => SortOrder::Ping,
            SortOrder::Ping => SortOrder::Killer,
        };
    }

    pub fn on_tick(&mut self, now: Instant) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        if let Some(notice) = &self.notice
            && let Some(expires_at) = notice.expires_at
            && now >= expires_at
        {
            self.notice = None;
        }
    }

    pub fn handle_key(&mut self, c: char) -> AppAction {
        let norm = normalize_key_char(c);
        if self.show_lock_modal {
            if norm == ' ' {
                self.lock_modal_toggle();
            }
            AppAction::None
        } else {
            match norm {
                's' => {
                    self.cycle_sort();
                    AppAction::SaveConfig(self.to_config())
                }
                'l' => {
                    self.open_lock_modal();
                    AppAction::None
                }
                'm' => {
                    self.mode = match self.mode {
                        GameMode::Standard => GameMode::Event,
                        GameMode::Event => GameMode::Standard,
                        GameMode::Both => GameMode::Standard,
                    };
                    self.clamp_selection();
                    AppAction::SaveConfig(self.to_config())
                }
                'r' => {
                    if !self.is_fetching {
                        self.is_fetching = true;
                        self.notice = None;
                        AppAction::Refresh
                    } else {
                        AppAction::None
                    }
                }
                _ => AppAction::None,
            }
        }
    }

    pub fn handle_up(&mut self) {
        if self.show_lock_modal {
            self.lock_modal_up();
        } else {
            self.previous();
        }
    }

    pub fn handle_down(&mut self) {
        if self.show_lock_modal {
            self.lock_modal_down();
        } else {
            self.next();
        }
    }

    pub fn handle_enter(&mut self) -> AppAction {
        if self.show_lock_modal {
            self.apply_lock_modal()
        } else {
            AppAction::None
        }
    }

    pub fn handle_esc(&mut self) {
        if self.show_lock_modal {
            self.cancel_lock_modal();
        } else {
            self.should_quit = true;
        }
    }

    pub fn handle_hosts_result(
        &mut self,
        res: crate::hosts::UpdateHostsResult,
        target_locked: Vec<String>,
        now: Instant,
    ) -> AppAction {
        let duration = std::time::Duration::from_secs(3);
        match res {
            crate::hosts::UpdateHostsResult::Updated => {
                self.locked = target_locked.into_iter().collect();
                let msg = i18n::tr(self.locale, i18n::TextKey::HostsUpdated);
                self.notice = Some(Notice::success(msg, now, duration));
                AppAction::SaveConfig(self.to_config())
            }
            crate::hosts::UpdateHostsResult::AlreadyUpToDate => {
                self.locked = target_locked.into_iter().collect();
                let msg = i18n::tr(self.locale, i18n::TextKey::HostsAlreadyUpToDate);
                self.notice = Some(Notice::info(msg, now, duration));
                AppAction::SaveConfig(self.to_config())
            }
            crate::hosts::UpdateHostsResult::ElevationFailed => {
                let msg = i18n::tr(self.locale, i18n::TextKey::HostsElevationFailed);
                self.notice = Some(Notice::error(msg));
                AppAction::None
            }
            crate::hosts::UpdateHostsResult::Error(e) => {
                let prefix = i18n::tr(self.locale, i18n::TextKey::ErrorPrefix);
                self.notice = Some(Notice::error(format!("{}{}", prefix, e)));
                AppAction::None
            }
        }
    }

    pub fn handle_manual_refresh_complete(
        &mut self,
        api_res: Result<(Vec<RegionQueueData>, i64), String>,
        ping_res: HashMap<String, u32>,
        now: Instant,
    ) {
        self.is_fetching = false;
        self.pings = ping_res;
        match api_res {
            Ok((queues, last_updated)) => {
                let is_same = self.api_last_updated == last_updated && !self.queues.is_empty();
                self.queues = queues;
                self.api_last_updated = last_updated;
                self.clamp_selection();
                let feedback = if is_same {
                    i18n::tr(self.locale, i18n::TextKey::FeedbackUpToDate)
                } else {
                    i18n::tr(self.locale, i18n::TextKey::FeedbackUpdated)
                };
                self.notice = Some(Notice::success(
                    feedback,
                    now,
                    std::time::Duration::from_secs(3),
                ));
            }
            Err(e) => {
                self.notice = Some(Notice::error(e));
            }
        }
    }

    pub fn handle_api_update(&mut self, res: Result<(Vec<RegionQueueData>, i64), String>) {
        match res {
            Ok((queues, last_updated)) => {
                self.queues = queues;
                self.api_last_updated = last_updated;
                self.clamp_selection();
                if let Some(notice) = &self.notice
                    && notice.kind == NoticeKind::Error
                {
                    self.notice = None;
                }
            }
            Err(e) => {
                if self.queues.is_empty() {
                    self.notice = Some(Notice::error(e));
                }
            }
        }
    }

    pub fn handle_ping_update(&mut self, pings: HashMap<String, u32>) {
        self.pings = pings;
    }

    pub fn get_filtered_sorted_rows(&self) -> Vec<&RegionQueueData> {
        let mut filtered: Vec<&RegionQueueData> = self
            .queues
            .iter()
            .filter(|r| match self.mode {
                GameMode::Standard => r.mode == "Standard",
                GameMode::Event => r.mode == "Event",
                GameMode::Both => true,
            })
            .collect();

        match self.sort {
            SortOrder::Survivor => {
                filtered.sort_by_cached_key(|r| {
                    (
                        r.is_disabled(),
                        api::parse_time_to_seconds(&r.survivor),
                        &r.name,
                    )
                });
            }
            SortOrder::Killer => {
                filtered.sort_by_cached_key(|r| {
                    (
                        r.is_disabled(),
                        api::parse_time_to_seconds(&r.killer),
                        &r.name,
                    )
                });
            }
            SortOrder::Ping => {
                let api_to_aws = api::get_api_to_aws();
                filtered.sort_by_cached_key(|r| {
                    let code = api_to_aws.get(r.name.as_str()).unwrap_or(&"");
                    let ping = self.pings.get(*code).copied().unwrap_or(u32::MAX);
                    (r.is_disabled(), ping, &r.name)
                });
            }
            SortOrder::Default => {
                filtered.sort_by_cached_key(|r| (r.is_disabled(), &r.name));
            }
        }
        filtered
    }

    pub fn summary(&self) -> Summary<'_> {
        let eligible_rows: Vec<&RegionQueueData> = self
            .get_filtered_sorted_rows()
            .into_iter()
            .filter(|r| !r.is_disabled())
            .collect();

        if eligible_rows.is_empty() {
            return Summary {
                killer: None,
                survivor: None,
                lowest_ping: None,
            };
        }

        let api_to_aws = api::get_api_to_aws();

        let get_ping = |row: &RegionQueueData| -> Option<u32> {
            let code = api_to_aws.get(row.name.as_str())?;
            self.pings.get(*code).copied()
        };

        let pick_best = |get_time_str: fn(&RegionQueueData) -> &str| -> Option<BestPick<'_>> {
            let mut min_secs = u32::MAX;
            for &row in &eligible_rows {
                let secs = api::parse_time_to_seconds(get_time_str(row));
                if secs < min_secs {
                    min_secs = secs;
                }
            }

            if min_secs >= 999999 {
                return None;
            }

            let max_secs = min_secs.saturating_add(NEAR_BEST_TOLERANCE_SECS);
            let candidates: Vec<&RegionQueueData> = eligible_rows
                .iter()
                .copied()
                .filter(|row| {
                    let secs = api::parse_time_to_seconds(get_time_str(row));
                    secs <= max_secs
                })
                .collect();

            if candidates.is_empty() {
                return None;
            }

            let best = candidates.iter().copied().min_by_key(|row| {
                let ping = get_ping(row).unwrap_or(u32::MAX);
                let secs = api::parse_time_to_seconds(get_time_str(row));
                (ping, secs, &row.name)
            })?;

            let similar = candidates.len() - 1;

            Some(BestPick { row: best, similar })
        };

        let killer = pick_best(|r| &r.killer);
        let survivor = pick_best(|r| &r.survivor);

        let lowest_ping = eligible_rows
            .iter()
            .copied()
            .filter_map(|row| get_ping(row).map(|ping| (ping, &row.name, row)))
            .min_by_key(|&(ping, name, _)| (ping, name))
            .map(|(_, _, row)| row);

        Summary {
            killer,
            survivor,
            lowest_ping,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_test_app() -> App {
        App::new(
            SortOrder::Default,
            GameMode::Standard,
            vec![],
            vec![],
            Language::En,
            None,
        )
    }

    #[test]
    fn test_normalize_key_char() {
        assert_eq!(normalize_key_char('д'), 'l');
        assert_eq!(normalize_key_char('Д'), 'l');
        assert_eq!(normalize_key_char('ы'), 's');
        assert_eq!(normalize_key_char('Ы'), 's');
        assert_eq!(normalize_key_char('л'), 'k');
        assert_eq!(normalize_key_char('Л'), 'k');
        assert_eq!(normalize_key_char('з'), 'p');
        assert_eq!(normalize_key_char('З'), 'p');
        assert_eq!(normalize_key_char('ь'), 'm');
        assert_eq!(normalize_key_char('Ь'), 'm');
        assert_eq!(normalize_key_char('L'), 'l');
        assert_eq!(normalize_key_char('l'), 'l');
    }

    #[test]
    fn test_sort_by_ping() {
        let mut app = make_test_app();
        app.sort = SortOrder::Ping;
        app.queues = vec![
            RegionQueueData {
                flag: "[US]".to_string(),
                name: "Virginia".to_string(),
                mode: "Standard".to_string(),
                survivor: "5s".to_string(),
                killer: "10s".to_string(),
            },
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "5s".to_string(),
                killer: "10s".to_string(),
            },
            RegionQueueData {
                flag: "[JP]".to_string(),
                name: "Tokyo".to_string(),
                mode: "Standard".to_string(),
                survivor: "5s".to_string(),
                killer: "10s".to_string(),
            },
        ];
        // Frankfurt: 30ms, Virginia: 110ms, Tokyo: unmeasured
        app.pings.insert("eu-central-1".to_string(), 30);
        app.pings.insert("us-east-1".to_string(), 110);

        let rows = app.get_filtered_sorted_rows();
        assert_eq!(rows[0].name, "Frankfurt");
        assert_eq!(rows[1].name, "Virginia");
        assert_eq!(rows[2].name, "Tokyo");
    }

    #[test]
    fn test_modal_regions_sorted_by_ping() {
        let mut app = make_test_app();
        app.pings.insert("eu-central-1".to_string(), 25);
        app.pings.insert("eu-west-1".to_string(), 45);
        app.pings.insert("us-east-1".to_string(), 105);

        let modal_regs = app.get_modal_regions();
        assert_eq!(modal_regs[0], "eu-central-1");
        assert_eq!(modal_regs[1], "eu-west-1");
        assert_eq!(modal_regs[2], "us-east-1");
    }

    #[test]
    fn test_modal_regions_filter_disabled() {
        let mut app = make_test_app();
        app.queues = vec![
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
        let modal_regs = app.get_modal_regions();
        assert!(!modal_regs.contains(&"eu-west-2")); // London is disabled
        assert!(modal_regs.contains(&"eu-central-1")); // Frankfurt is active
    }

    #[test]
    fn test_disabled_servers_at_bottom() {
        let mut app = make_test_app();
        app.sort = SortOrder::Ping;
        app.queues = vec![
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "—".to_string(),
                killer: "—".to_string(),
            },
            RegionQueueData {
                flag: "[US]".to_string(),
                name: "Virginia".to_string(),
                mode: "Standard".to_string(),
                survivor: "15s".to_string(),
                killer: "45s".to_string(),
            },
            RegionQueueData {
                flag: "[IE]".to_string(),
                name: "Dublin".to_string(),
                mode: "Standard".to_string(),
                survivor: "2m".to_string(),
                killer: "1m".to_string(),
            },
        ];
        app.pings.insert("eu-central-1".to_string(), 15); // Frankfurt (disabled)
        app.pings.insert("us-east-1".to_string(), 80); // Virginia (active)
        app.pings.insert("eu-west-1".to_string(), 40); // Dublin (active)

        let rows = app.get_filtered_sorted_rows();
        assert_eq!(rows[0].name, "Dublin");
        assert_eq!(rows[1].name, "Virginia");
        assert_eq!(rows[2].name, "Frankfurt");
    }

    #[test]
    fn test_cycle_sort() {
        let mut app = make_test_app();
        app.sort = SortOrder::Killer;
        app.cycle_sort();
        assert_eq!(app.sort, SortOrder::Survivor);
        app.cycle_sort();
        assert_eq!(app.sort, SortOrder::Ping);
        app.cycle_sort();
        assert_eq!(app.sort, SortOrder::Killer);

        // From default
        app.sort = SortOrder::Default;
        app.cycle_sort();
        assert_eq!(app.sort, SortOrder::Killer);
    }

    #[test]
    fn test_actions_returned_instead_of_io() {
        let mut app = make_test_app();

        // Sort key returns SaveConfig
        let action = app.handle_key('s');
        match action {
            AppAction::SaveConfig(cfg) => assert_eq!(cfg.sort, SortOrder::Killer),
            other => panic!("Expected SaveConfig, got {:?}", other),
        }

        // Mode key returns SaveConfig
        let action = app.handle_key('m');
        match action {
            AppAction::SaveConfig(cfg) => assert_eq!(cfg.mode, GameMode::Event),
            other => panic!("Expected SaveConfig, got {:?}", other),
        }

        // Refresh key returns Refresh
        app.is_fetching = false;
        let action = app.handle_key('r');
        assert_eq!(action, AppAction::Refresh);

        // Applying lock modal returns ApplyLocks without modifying app.locked
        app.open_lock_modal();
        app.lock_modal_selected = vec!["eu-central-1".to_string()];
        let action = app.handle_enter();
        match action {
            AppAction::ApplyLocks(regions) => assert_eq!(regions, vec!["eu-central-1"]),
            other => panic!("Expected ApplyLocks, got {:?}", other),
        }
        // App's locked set is still empty until hosts result comes back
        assert!(app.locked.is_empty());
    }

    #[test]
    fn test_hosts_then_config_ordering() {
        let mut app = make_test_app();
        let now = Instant::now();

        // Success: update_hosts succeeds -> locked updated, success notice, SaveConfig action returned
        let action = app.handle_hosts_result(
            crate::hosts::UpdateHostsResult::Updated,
            vec!["eu-central-1".to_string()],
            now,
        );
        assert!(app.locked.contains("eu-central-1"));
        assert!(app.notice.as_ref().map(|n| n.kind) == Some(NoticeKind::Success));
        match action {
            AppAction::SaveConfig(cfg) => assert_eq!(cfg.locked, vec!["eu-central-1"]),
            other => panic!("Expected SaveConfig, got {:?}", other),
        }

        // Failure: elevation denied -> locked preserved, error notice, AppAction::None
        let action2 = app.handle_hosts_result(
            crate::hosts::UpdateHostsResult::ElevationFailed,
            vec!["us-east-1".to_string()],
            now,
        );
        assert!(app.locked.contains("eu-central-1"));
        assert!(!app.locked.contains("us-east-1"));
        assert!(app.notice.as_ref().map(|n| n.kind) == Some(NoticeKind::Error));
        assert_eq!(action2, AppAction::None);

        // Failure: generic error -> locked preserved, error notice, AppAction::None
        let action3 = app.handle_hosts_result(
            crate::hosts::UpdateHostsResult::Error("write error".to_string()),
            vec!["us-east-1".to_string()],
            now,
        );
        assert!(app.locked.contains("eu-central-1"));
        assert!(!app.locked.contains("us-east-1"));
        assert!(app.notice.as_ref().map(|n| n.kind) == Some(NoticeKind::Error));
        assert_eq!(action3, AppAction::None);
    }

    #[test]
    fn test_notice_expiry_injected_time() {
        let mut app = make_test_app();
        let start = Instant::now();

        // Set info notice with 3 second expiry
        app.notice = Some(Notice::info("Info message", start, Duration::from_secs(3)));

        // Tick at start + 1s: still active
        app.on_tick(start + Duration::from_secs(1));
        assert!(app.notice.is_some());

        // Tick at start + 3s: expired
        app.on_tick(start + Duration::from_secs(3));
        assert!(app.notice.is_none());

        // Error notice: has no expiry, should persist across ticks
        app.notice = Some(Notice::error("Persistent error"));
        app.on_tick(start + Duration::from_secs(10));
        assert!(app.notice.is_some());
        assert_eq!(app.notice.as_ref().unwrap().kind, NoticeKind::Error);
    }

    #[test]
    fn test_selection_clamping() {
        let mut app = make_test_app();
        app.queues = vec![
            RegionQueueData {
                flag: "[US]".to_string(),
                name: "Virginia".to_string(),
                mode: "Standard".to_string(),
                survivor: "5s".to_string(),
                killer: "10s".to_string(),
            },
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "5s".to_string(),
                killer: "10s".to_string(),
            },
        ];

        app.table_state.select(Some(1));
        assert_eq!(app.table_state.selected(), Some(1));

        // Queues cleared -> selection clamped to None
        app.queues.clear();
        app.clamp_selection();
        assert_eq!(app.table_state.selected(), None);

        // Modal cursor clamping
        app.lock_modal_cursor = 100;
        app.clamp_selection();
        let modal_len = app.get_modal_regions().len();
        assert!(app.lock_modal_cursor < modal_len);
    }

    #[test]
    fn test_move_selection_wrap() {
        let mut app = make_test_app();
        app.queues = vec![
            RegionQueueData {
                flag: "[US]".to_string(),
                name: "Virginia".to_string(),
                mode: "Standard".to_string(),
                survivor: "5s".to_string(),
                killer: "10s".to_string(),
            },
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "5s".to_string(),
                killer: "10s".to_string(),
            },
        ];

        app.table_state.select(Some(0));
        app.move_selection(Direction::Up);
        assert_eq!(app.table_state.selected(), Some(1)); // wrapped to bottom

        app.move_selection(Direction::Down);
        assert_eq!(app.table_state.selected(), Some(0)); // wrapped to top
    }

    #[test]
    fn test_summary_four_tied_lowest_ping_wins() {
        // (a) four regions tied at 6s killer with different pings → lowest ping wins, similar == 3;
        let mut app = make_test_app();
        app.queues = vec![
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "6s".to_string(),
            },
            RegionQueueData {
                flag: "[IE]".to_string(),
                name: "Dublin".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "6s".to_string(),
            },
            RegionQueueData {
                flag: "[GB]".to_string(),
                name: "London".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "6s".to_string(),
            },
            RegionQueueData {
                flag: "[US]".to_string(),
                name: "Virginia".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "6s".to_string(),
            },
        ];
        // Frankfurt: 50ms, Dublin: 25ms, London: 60ms, Virginia: 110ms
        app.pings.insert("eu-central-1".to_string(), 50);
        app.pings.insert("eu-west-1".to_string(), 25);
        app.pings.insert("eu-west-2".to_string(), 60);
        app.pings.insert("us-east-1".to_string(), 110);

        let summary = app.summary();
        let killer_pick = summary.killer.expect("killer pick should be present");
        assert_eq!(killer_pick.row.name, "Dublin");
        assert_eq!(killer_pick.similar, 3);
        assert_eq!(summary.lowest_ping.unwrap().name, "Dublin");
    }

    #[test]
    fn test_summary_within_tolerance_lower_ping_wins() {
        // (b) 6s region with 296 ms vs 9s region with 40 ms → the 9s region wins (within tolerance), similar == 1;
        let mut app = make_test_app();
        app.queues = vec![
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "6s".to_string(),
            },
            RegionQueueData {
                flag: "[IE]".to_string(),
                name: "Dublin".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "9s".to_string(),
            },
        ];
        app.pings.insert("eu-central-1".to_string(), 296);
        app.pings.insert("eu-west-1".to_string(), 40);

        let summary = app.summary();
        let killer_pick = summary.killer.expect("killer pick should be present");
        assert_eq!(killer_pick.row.name, "Dublin");
        assert_eq!(killer_pick.similar, 1);
    }

    #[test]
    fn test_summary_outside_tolerance_lower_ping_loses() {
        // (c) 6s vs 30s → 6s wins even if 30s has lower ping (outside tolerance), similar == 0;
        let mut app = make_test_app();
        app.queues = vec![
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "6s".to_string(),
            },
            RegionQueueData {
                flag: "[IE]".to_string(),
                name: "Dublin".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "30s".to_string(),
            },
        ];
        app.pings.insert("eu-central-1".to_string(), 100);
        app.pings.insert("eu-west-1".to_string(), 20);

        let summary = app.summary();
        let killer_pick = summary.killer.expect("killer pick should be present");
        assert_eq!(killer_pick.row.name, "Frankfurt");
        assert_eq!(killer_pick.similar, 0);
    }

    #[test]
    fn test_summary_disabled_row_ignored() {
        // (d) a disabled row ("—"/"—") is ignored even with the lowest ping;
        let mut app = make_test_app();
        app.queues = vec![
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "—".to_string(),
                killer: "—".to_string(),
            },
            RegionQueueData {
                flag: "[IE]".to_string(),
                name: "Dublin".to_string(),
                mode: "Standard".to_string(),
                survivor: "15s".to_string(),
                killer: "15s".to_string(),
            },
        ];
        app.pings.insert("eu-central-1".to_string(), 10);
        app.pings.insert("eu-west-1".to_string(), 50);

        let summary = app.summary();
        let killer_pick = summary.killer.expect("killer pick should be present");
        assert_eq!(killer_pick.row.name, "Dublin");
        assert_eq!(killer_pick.similar, 0);
        assert_eq!(summary.lowest_ping.unwrap().name, "Dublin");
    }

    #[test]
    fn test_summary_unmeasured_ping_loses_to_measured() {
        // (e) region at 6s with no measured ping loses to a measured 6s region;
        let mut app = make_test_app();
        app.queues = vec![
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "6s".to_string(),
            },
            RegionQueueData {
                flag: "[IE]".to_string(),
                name: "Dublin".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "6s".to_string(),
            },
        ];
        // Only Frankfurt has a measured ping
        app.pings.insert("eu-central-1".to_string(), 40);

        let summary = app.summary();
        let killer_pick = summary.killer.expect("killer pick should be present");
        assert_eq!(killer_pick.row.name, "Frankfurt");
        assert_eq!(killer_pick.similar, 1);
    }

    #[test]
    fn test_summary_respects_mode_filter() {
        // (f) mode filter is respected (Event rows ignored when mode == Standard).
        let mut app = make_test_app();
        app.mode = GameMode::Standard;
        app.queues = vec![
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Event".to_string(),
                survivor: "2s".to_string(),
                killer: "2s".to_string(),
            },
            RegionQueueData {
                flag: "[IE]".to_string(),
                name: "Dublin".to_string(),
                mode: "Standard".to_string(),
                survivor: "15s".to_string(),
                killer: "15s".to_string(),
            },
        ];
        app.pings.insert("eu-central-1".to_string(), 10);
        app.pings.insert("eu-west-1".to_string(), 50);

        let summary = app.summary();
        let killer_pick = summary.killer.expect("killer pick should be present");
        assert_eq!(killer_pick.row.name, "Dublin");
        assert_eq!(summary.lowest_ping.unwrap().name, "Dublin");
    }
}
