use crate::api::{self, RegionQueueData};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, Cell, Row, Table, Paragraph, TableState, Clear, List, ListItem},
    Frame,
};
use std::collections::HashMap;

use crate::config::{AppConfig, get_config_path, save_config};

fn is_russian() -> bool {
    let lang = std::env::var("LANG").unwrap_or_default().to_lowercase();
    let lc_all = std::env::var("LC_ALL").unwrap_or_default().to_lowercase();
    let lc_msg = std::env::var("LC_MESSAGES").unwrap_or_default().to_lowercase();
    
    lang.starts_with("ru") || lc_all.starts_with("ru") || lc_msg.starts_with("ru")
}

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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AppAction {
    None,
    Refresh,
}

pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct App {
    pub queues: Vec<RegionQueueData>,
    pub api_last_updated: i64,
    pub pings: HashMap<String, u32>,
    pub sort: String,
    pub mode: String,
    pub priority: Vec<String>,
    pub locked: Vec<String>,
    pub error_msg: Option<String>,
    pub status_msg: Option<String>,
    pub should_quit: bool,
    pub is_fetching: bool,
    pub table_state: TableState,
    pub show_lock_modal: bool,
    pub lock_modal_selected: Vec<String>,
    pub lock_modal_cursor: usize,
    pub spinner_frame: usize,
    pub refresh_feedback: Option<(String, std::time::Instant)>,
}

impl App {
    pub fn new(sort: String, mode: String, priority: Vec<String>, locked: Vec<String>) -> Self {
        Self {
            queues: Vec::new(),
            api_last_updated: 0,
            pings: HashMap::new(),
            sort,
            mode,
            priority,
            locked,
            error_msg: None,
            status_msg: None,
            should_quit: false,
            is_fetching: true,
            table_state: TableState::default(),
            show_lock_modal: false,
            lock_modal_selected: Vec::new(),
            lock_modal_cursor: 0,
            spinner_frame: 0,
            refresh_feedback: None,
        }
    }

    pub fn next(&mut self) {
        let total = self.get_filtered_sorted_rows().len();
        if total == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= total.saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let total = self.get_filtered_sorted_rows().len();
        if total == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    total.saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn get_modal_regions(&self) -> Vec<&'static str> {
        let disabled = crate::api::get_disabled_aws_regions(&self.queues);
        let mut regions: Vec<&'static str> = crate::api::get_all_aws_regions()
            .into_iter()
            .filter(|reg| !disabled.contains(*reg))
            .collect();
        regions.sort_by(|&a, &b| {
            let a_ping = self.pings.get(a).copied().unwrap_or(u32::MAX);
            let b_ping = self.pings.get(b).copied().unwrap_or(u32::MAX);
            a_ping.cmp(&b_ping).then_with(|| {
                let aws_to_api = crate::api::get_aws_to_api();
                let a_name = aws_to_api.get(a).unwrap_or(&a);
                let b_name = aws_to_api.get(b).unwrap_or(&b);
                a_name.cmp(b_name)
            })
        });
        regions
    }

    pub fn open_lock_modal(&mut self) {
        self.show_lock_modal = true;
        self.lock_modal_selected = self.locked.clone();
        self.lock_modal_cursor = 0;
    }

    pub fn lock_modal_up(&mut self) {
        let regions = self.get_modal_regions();
        if self.lock_modal_cursor == 0 {
            self.lock_modal_cursor = regions.len().saturating_sub(1);
        } else {
            self.lock_modal_cursor -= 1;
        }
    }

    pub fn lock_modal_down(&mut self) {
        let regions = self.get_modal_regions();
        if self.lock_modal_cursor >= regions.len().saturating_sub(1) {
            self.lock_modal_cursor = 0;
        } else {
            self.lock_modal_cursor += 1;
        }
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

    pub fn apply_lock_modal(&mut self) {
        self.show_lock_modal = false;
        self.locked = self.lock_modal_selected.clone();
        self.save_current_config();
        
        let lock_target = if self.locked.is_empty() {
            None
        } else {
            Some(self.locked.as_slice())
        };
        let res = crate::hosts::update_hosts(lock_target, false);
        let is_ru = is_russian();
        self.status_msg = Some(match res {
            crate::hosts::UpdateHostsResult::Updated => {
                if is_ru { "Блокировка регионов обновлена!".to_string() } else { "Region locks updated!".to_string() }
            }
            crate::hosts::UpdateHostsResult::AlreadyUpToDate => {
                if is_ru { "Файл hosts уже актуален".to_string() } else { "Hosts file is up to date".to_string() }
            }
            crate::hosts::UpdateHostsResult::ElevationFailed => {
                if is_ru { "Ошибка: отказ в правах Администратора".to_string() } else { "Error: admin elevation denied".to_string() }
            }
            crate::hosts::UpdateHostsResult::Error(e) => {
                if is_ru { format!("Ошибка: {}", e) } else { format!("Error: {}", e) }
            }
        });
    }

    pub fn cancel_lock_modal(&mut self) {
        self.show_lock_modal = false;
    }

    fn save_current_config(&self) {
        let config_path = get_config_path();
        let existing = crate::config::load_config(&config_path);
        let config = AppConfig {
            priority: self.priority.clone(),
            locked: self.locked.clone(),
            sort: self.sort.clone(),
            mode: self.mode.clone(),
            api_url: existing.api_url,
        };
        let _ = save_config(&config_path, &config);
    }

    #[allow(dead_code)]
    pub fn set_sort(&mut self, sort: &str) {
        self.sort = sort.to_string();
        self.save_current_config();
    }

    pub fn cycle_sort(&mut self) {
        self.sort = match self.sort.as_str() {
            "killer" => "survivor".to_string(),
            "survivor" => "ping".to_string(),
            "ping" => "killer".to_string(),
            _ => "killer".to_string(),
        };
        self.save_current_config();
    }

    pub fn on_tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        if let Some((_, instant)) = &self.refresh_feedback {
            if instant.elapsed() >= std::time::Duration::from_millis(2500) {
                self.refresh_feedback = None;
            }
        }
    }

    pub fn handle_key(&mut self, c: char) -> AppAction {
        let norm = normalize_key_char(c);
        if self.show_lock_modal {
            match norm {
                ' ' => self.lock_modal_toggle(),
                _ => {}
            }
            AppAction::None
        } else {
            match norm {
                's' => { self.cycle_sort(); AppAction::None }
                'l' => { self.open_lock_modal(); AppAction::None }
                'm' => {
                    if self.mode == "standard" {
                        self.mode = "event".to_string();
                    } else {
                        self.mode = "standard".to_string();
                    }
                    self.save_current_config();
                    AppAction::None
                }
                'r' => {
                    if !self.is_fetching {
                        self.is_fetching = true;
                        self.error_msg = None;
                        self.status_msg = None;
                        self.refresh_feedback = None;
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

    pub fn handle_enter(&mut self) {
        if self.show_lock_modal {
            self.apply_lock_modal();
        }
    }

    pub fn handle_esc(&mut self) {
        if self.show_lock_modal {
            self.cancel_lock_modal();
        } else {
            self.should_quit = true;
        }
    }

    pub fn get_filtered_sorted_rows(&self) -> Vec<RegionQueueData> {
        let api_to_aws = crate::api::get_api_to_aws();
        let mut filtered: Vec<RegionQueueData> = self.queues.iter().filter(|r| {
            if self.mode == "standard" { r.mode == "Standard" }
            else if self.mode == "event" { r.mode == "Event" }
            else { r.mode == "Standard" }
        }).cloned().collect();

        filtered.sort_by(|a, b| {
            let a_disabled = a.survivor == "—" && a.killer == "—";
            let b_disabled = b.survivor == "—" && b.killer == "—";

            if a_disabled != b_disabled {
                return a_disabled.cmp(&b_disabled);
            }

            match self.sort.as_str() {
                "survivor" => {
                    let a_time = api::parse_time_to_seconds(&a.survivor);
                    let b_time = api::parse_time_to_seconds(&b.survivor);
                    a_time.cmp(&b_time).then_with(|| a.name.cmp(&b.name))
                }
                "killer" => {
                    let a_time = api::parse_time_to_seconds(&a.killer);
                    let b_time = api::parse_time_to_seconds(&b.killer);
                    a_time.cmp(&b_time).then_with(|| a.name.cmp(&b.name))
                }
                "ping" => {
                    let a_code = api_to_aws.get(a.name.as_str()).unwrap_or(&"");
                    let b_code = api_to_aws.get(b.name.as_str()).unwrap_or(&"");
                    let a_ping = self.pings.get(*a_code).copied().unwrap_or(u32::MAX);
                    let b_ping = self.pings.get(*b_code).copied().unwrap_or(u32::MAX);
                    a_ping.cmp(&b_ping).then_with(|| a.name.cmp(&b.name))
                }
                _ => a.name.cmp(&b.name),
            }
        });
        filtered
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    // Fill background with spaces to completely fix visual ghosting when table shrinks
    f.render_widget(Clear, f.area());

    // Get rows first so we can dynamically size the table block
    let rows_data = app.get_filtered_sorted_rows();
    let is_empty = rows_data.is_empty();
    let table_height = if is_empty {
        6 // Header (1), margin (1), border (2), placeholder row (1), bottom buffer (1)
    } else {
        (rows_data.len() as u16) + 4 // 1 header, 1 margin, 2 borders
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Header/Title
                Constraint::Length(table_height), // Table (tight fit)
                Constraint::Min(0), // Empty space
                Constraint::Length(1), // Footer/Status
            ]
            .as_ref(),
        )
        .split(f.area());

    let is_ru = is_russian();
    let title_text = " Dead By Queue ";

    let active_sort_str = match app.sort.as_str() {
        "killer" => if is_ru { "Маньяк" } else { "Killer" },
        "survivor" => if is_ru { "Выживший" } else { "Survivor" },
        "ping" => if is_ru { "Пинг" } else { "Ping" },
        _ => if is_ru { "Маньяк" } else { "Killer" },
    };

    let mode_str = if is_ru {
        match app.mode.as_str() {
            "event" => "Ивент",
            _ => "Обычный",
        }
    } else {
        match app.mode.as_str() {
            "event" => "Event",
            _ => "Standard",
        }
    };

    let lock_str = if app.locked.is_empty() {
        if is_ru { "Все" } else { "None" }
    } else {
        if is_ru { "Активен" } else { "Active" }
    };

    // Header block
    let header_line = Line::from(vec![
        Span::styled(if is_ru { "Сортировка: " } else { "Sort: " }, Style::default().fg(Color::DarkGray)),
        Span::styled(active_sort_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
        Span::styled(if is_ru { "Режим: " } else { "Mode: " }, Style::default().fg(Color::DarkGray)),
        Span::styled(mode_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
        Span::styled(if is_ru { "Блокировка: " } else { "Lock: " }, Style::default().fg(Color::DarkGray)),
        Span::styled(
            lock_str,
            if !app.locked.is_empty() {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
    ]);

    let header = Paragraph::new(header_line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::LightRed))
                .title(Span::styled(title_text, Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD))),
        );
    f.render_widget(header, chunks[0]);

    // Table
    let api_to_aws = crate::api::get_api_to_aws();
    let (rows, col_constraints) = if is_empty {
        app.table_state.select(None);
        let (msg, color) = if app.is_fetching {
            (if is_ru { "  Загрузка данных очередей..." } else { "  Fetching queue times..." }, Color::LightRed)
        } else if app.error_msg.is_some() {
            (if is_ru { "  Не удалось загрузить данные очередей. Проверьте сеть или прокси. Нажмите 'R' для повтора." } else { "  Failed to load queue data. Check network or proxy. Press 'R' to retry." }, Color::Red)
        } else {
            (if is_ru { "  Нет данных для выбранного режима." } else { "  No queue data available for this mode." }, Color::DarkGray)
        };
        (
            vec![Row::new(vec![
                Cell::from(Span::styled(msg, Style::default().fg(color))),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
            ])],
            [
                Constraint::Percentage(100),
                Constraint::Length(0),
                Constraint::Length(0),
                Constraint::Length(0),
            ],
        )
    } else {
        let r: Vec<Row> = rows_data.into_iter().map(|item| {
            let aws_code = api_to_aws.get(item.name.as_str()).unwrap_or(&"");
            let is_whitelisted = app.locked.iter().any(|l| l == aws_code);
            let is_disabled = item.survivor == "—" && item.killer == "—";

            let reg_str = if item.flag.is_empty() {
                item.name.clone()
            } else {
                format!("{} {}", item.flag, item.name)
            };

            if is_disabled {
                let dim_style = Style::default().fg(Color::DarkGray);

                Row::new(vec![
                    Cell::from(Span::styled(reg_str, dim_style)),
                    Cell::from(Span::styled("—", dim_style)),
                    Cell::from(Span::styled("—", dim_style)),
                    Cell::from(Span::styled("—", dim_style)),
                ])
            } else {
                let name_style = if is_whitelisted {
                    Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let (ping_str, ping_color) = if let Some(&ms) = app.pings.get(*aws_code) {
                    (format!("{} ms", ms), crate::ping::color_for_ping(Some(ms)))
                } else if app.pings.is_empty() {
                    ("...".to_string(), Color::DarkGray)
                } else {
                    ("—".to_string(), Color::DarkGray)
                };

                let surv_color = color_for_time(&item.survivor);
                let kill_color = color_for_time(&item.killer);

                Row::new(vec![
                    Cell::from(Span::styled(reg_str, name_style)),
                    Cell::from(Span::styled(ping_str, Style::default().fg(ping_color))),
                    Cell::from(Span::styled(item.survivor.clone(), Style::default().fg(surv_color))),
                    Cell::from(Span::styled(item.killer.clone(), Style::default().fg(kill_color))),
                ])
            }
        }).collect();
        (
            r,
            [
                Constraint::Min(24),
                Constraint::Length(12),
                Constraint::Length(14),
                Constraint::Length(14),
            ],
        )
    };

    let hdr_region = if is_ru { "Регион" } else { "Region" };
    let hdr_ping = if is_ru { "Пинг" } else { "Ping" };
    let hdr_survivor = if is_ru { "Выживший" } else { "Survivor" };
    let hdr_killer = if is_ru { "Маньяк" } else { "Killer" };

    let mut table = Table::new(rows, col_constraints)
        .header(
            Row::new(vec![hdr_region, hdr_ping, hdr_survivor, hdr_killer])
                .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
        .column_spacing(1);

    if !is_empty {
        table = table.row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    }

    f.render_stateful_widget(table, chunks[1], &mut app.table_state);

    // Footer
    let now = chrono::Utc::now().timestamp();
    let diff = now - app.api_last_updated;
    let time_str = if app.api_last_updated == 0 {
        if is_ru { "Загрузка...".to_string() } else { "fetching...".to_string() }
    } else if diff < 60 {
        if is_ru { "только что".to_string() } else { "just now".to_string() }
    } else if diff < 3600 {
        if is_ru { format!("{} мин. назад", diff / 60) } else { format!("{}m ago", diff / 60) }
    } else {
        if is_ru { format!("{} ч. {} мин. назад", diff / 3600, (diff % 3600) / 60) } else { format!("{}h {}m ago", diff / 3600, (diff % 3600) / 60) }
    };

    let status_span = if let Some(ref err) = app.error_msg {
        Span::styled(format!("Error: {}", err), Style::default().fg(Color::Red))
    } else if let Some(ref status) = app.status_msg {
        Span::styled(status.clone(), Style::default().fg(Color::LightRed))
    } else if app.is_fetching {
        let spinner = SPINNER_FRAMES[app.spinner_frame % SPINNER_FRAMES.len()];
        let msg = if is_ru { "Обновление..." } else { "Fetching..." };
        Span::styled(format!("{} {}", spinner, msg), Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD))
    } else if let Some((ref feedback, _)) = app.refresh_feedback {
        Span::styled(feedback.clone(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        let api_updated = if is_ru {
            format!("API обновлено: {}", time_str)
        } else {
            format!("API Updated: {}", time_str)
        };
        Span::styled(api_updated, Style::default().fg(Color::DarkGray))
    };

    let scroll_txt = if is_ru { "Прокрутка " } else { "Scroll " };
    let lock_txt = if is_ru { "Блокировка " } else { "Lock " };
    let sort_txt = if is_ru { "Сортировка " } else { "Sort " };
    let mode_txt = if is_ru { "Режим " } else { "Mode " };
    let refresh_txt = if is_ru { "Обновить " } else { "Refresh " };
    let quit_txt = if is_ru { "Выход " } else { "Quit " };

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" [\u{2191}\u{2193}] ", Style::default().fg(Color::LightRed)),
        Span::raw(scroll_txt),
        Span::styled(" [L] ", Style::default().fg(Color::LightRed)),
        Span::raw(lock_txt),
        Span::styled(" [S] ", Style::default().fg(Color::LightRed)),
        Span::raw(sort_txt),
        Span::styled(" [M] ", Style::default().fg(Color::LightRed)),
        Span::raw(mode_txt),
        Span::styled(" [R] ", Style::default().fg(Color::LightRed)),
        Span::raw(refresh_txt),
        Span::styled(" [Esc] ", Style::default().fg(Color::LightRed)),
        Span::raw(quit_txt),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        status_span,
    ]));
    f.render_widget(footer, chunks[3]);

    if app.show_lock_modal {
        let area = centered_rect(65, 80, f.area());
        f.render_widget(Clear, area);
        
        let modal_title = if is_ru {
            " Блокировка регионов "
        } else {
            " Region Locker "
        };
        
        let (select_txt, toggle_txt, apply_txt, cancel_txt) = if is_ru {
            ("Выбор", "Вкл/Выкл", "Применить", "Отмена")
        } else {
            ("Select", "Toggle", "Apply", "Cancel")
        };

        let modal_instructions = Line::from(vec![
            Span::styled(" [↑↓] ", Style::default().fg(Color::LightRed)),
            Span::raw(select_txt),
            Span::styled("  [Space] ", Style::default().fg(Color::LightRed)),
            Span::raw(toggle_txt),
            Span::styled("  [Enter] ", Style::default().fg(Color::LightRed)),
            Span::raw(apply_txt),
            Span::styled("  [Esc] ", Style::default().fg(Color::LightRed)),
            Span::raw(cancel_txt),
        ]);
        
        let modal_regions = app.get_modal_regions();
        let aws_to_api = crate::api::get_aws_to_api();
        let aws_to_flag = crate::api::get_aws_to_flag();
        
        let items: Vec<ListItem> = modal_regions.iter().enumerate().map(|(idx, code)| {
            let name = aws_to_api.get(*code).unwrap_or(code);
            let flag = aws_to_flag.get(*code).unwrap_or(&"");
            let flag_str = if flag.is_empty() { String::new() } else { format!("{} ", flag) };
            let is_selected = app.lock_modal_selected.iter().any(|r| r == *code);
            
            let checkbox = if is_selected { "[*] " } else { "[ ] " };
            
            let ping_str = if let Some(&ms) = app.pings.get(*code) {
                format!(" - {} ms", ms)
            } else {
                String::new()
            };
            
            let text = format!("{}{}{} ({}){}", checkbox, flag_str, name, code, ping_str);
            
            let mut style = if is_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            
            if idx == app.lock_modal_cursor {
                style = style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
            }
            
            ListItem::new(text).style(style)
        }).collect();
        
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::LightRed))
                    .title(Span::styled(modal_title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)))
                    .title_bottom(modal_instructions)
            );
        f.render_widget(list, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn color_for_time(time_str: &str) -> Color {
    if time_str == "—" || time_str.is_empty() {
        return Color::DarkGray;
    }
    let sec = api::parse_time_to_seconds(time_str);
    if sec < 60 {
        Color::Green
    } else if sec < 180 {
        Color::Yellow
    } else {
        Color::Red
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut app = App::new("ping".to_string(), "standard".to_string(), vec![], vec![]);
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
        let mut app = App::new("default".to_string(), "standard".to_string(), vec![], vec![]);
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
        let mut app = App::new("default".to_string(), "standard".to_string(), vec![], vec![]);
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
        let mut app = App::new("ping".to_string(), "standard".to_string(), vec![], vec![]);
        app.queues = vec![
            // Disabled server with low ping (15ms)
            RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "—".to_string(),
                killer: "—".to_string(),
            },
            // Active server with higher ping (80ms)
            RegionQueueData {
                flag: "[US]".to_string(),
                name: "Virginia".to_string(),
                mode: "Standard".to_string(),
                survivor: "15s".to_string(),
                killer: "45s".to_string(),
            },
            // Active server with moderate ping (40ms)
            RegionQueueData {
                flag: "[IE]".to_string(),
                name: "Dublin".to_string(),
                mode: "Standard".to_string(),
                survivor: "2m".to_string(),
                killer: "1m".to_string(),
            },
        ];
        app.pings.insert("eu-central-1".to_string(), 15); // Frankfurt (disabled)
        app.pings.insert("us-east-1".to_string(), 80);    // Virginia (active)
        app.pings.insert("eu-west-1".to_string(), 40);    // Dublin (active)

        let rows = app.get_filtered_sorted_rows();
        // Active servers sorted by ping: Dublin (40ms), Virginia (80ms)
        // Disabled server at the bottom: Frankfurt (15ms)
        assert_eq!(rows[0].name, "Dublin");
        assert_eq!(rows[1].name, "Virginia");
        assert_eq!(rows[2].name, "Frankfurt");
    }

    #[test]
    fn test_cycle_sort() {
        let mut app = App::new("killer".to_string(), "standard".to_string(), vec![], vec![]);
        app.cycle_sort();
        assert_eq!(app.sort, "survivor");
        app.cycle_sort();
        assert_eq!(app.sort, "ping");
        app.cycle_sort();
        assert_eq!(app.sort, "killer");

        // Fallback from default
        app.sort = "default".to_string();
        app.cycle_sort();
        assert_eq!(app.sort, "killer");
    }

    #[test]
    fn test_on_tick_spinner_and_feedback() {
        let mut app = App::new("killer".to_string(), "standard".to_string(), vec![], vec![]);
        assert_eq!(app.spinner_frame, 0);
        app.on_tick();
        assert_eq!(app.spinner_frame, 1);

        // Test feedback expiration: simulate instant in the past
        let past = std::time::Instant::now() - std::time::Duration::from_millis(3000);
        app.refresh_feedback = Some(("[✓ Up to date]".to_string(), past));
        app.on_tick();
        assert!(app.refresh_feedback.is_none());

        // Test feedback active: fresh instant
        let fresh = std::time::Instant::now();
        app.refresh_feedback = Some(("[✓ Up to date]".to_string(), fresh));
        app.on_tick();
        assert!(app.refresh_feedback.is_some());
    }
}

