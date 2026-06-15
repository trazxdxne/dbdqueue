use crate::api::{self, RegionQueueData};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, Cell, Row, Table, Paragraph, TableState, Clear},
    Frame,
};

use crate::config::{AppConfig, get_config_path, save_config};

fn is_russian() -> bool {
    let lang = std::env::var("LANG").unwrap_or_default().to_lowercase();
    let lc_all = std::env::var("LC_ALL").unwrap_or_default().to_lowercase();
    let lc_msg = std::env::var("LC_MESSAGES").unwrap_or_default().to_lowercase();
    
    lang.starts_with("ru") || lc_all.starts_with("ru") || lc_msg.starts_with("ru")
}

pub struct App {
    pub queues: Vec<RegionQueueData>,
    pub api_last_updated: i64,
    pub sort: String,
    pub mode: String,
    pub priority: Vec<String>,
    pub error_msg: Option<String>,
    pub should_quit: bool,
    pub is_fetching: bool,
    pub table_state: TableState,
}

impl App {
    pub fn new(sort: String, mode: String, priority: Vec<String>) -> Self {
        Self {
            queues: Vec::new(),
            api_last_updated: 0,
            sort,
            mode,
            priority,
            error_msg: None,
            should_quit: false,
            is_fetching: true,
            table_state: TableState::default(),
        }
    }

    pub fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.get_filtered_sorted_rows().len().saturating_sub(1) {
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
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.get_filtered_sorted_rows().len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn handle_key(&mut self, c: char) {
        match c {
            'q' => self.should_quit = true,
            's' => self.sort = "survivor".to_string(),
            'k' => self.sort = "killer".to_string(),
            'r' => self.sort = "priority".to_string(),
            'p' => self.toggle_priority(),
            'm' => {
                if self.mode == "standard" {
                    self.mode = "event".to_string();
                } else {
                    self.mode = "standard".to_string();
                }
            }
            _ => {}
        }
    }

    pub fn toggle_priority(&mut self) {
        if let Some(i) = self.table_state.selected() {
            let rows = self.get_filtered_sorted_rows();
            if let Some(row) = rows.get(i) {
                let name = row.name.clone();
                if let Some(pos) = self.priority.iter().position(|p| p == &name) {
                    self.priority.remove(pos);
                } else {
                    self.priority.push(name);
                }
                
                // Save to config
                let config_path = get_config_path();
                let config = AppConfig {
                    priority: self.priority.clone(),
                    sort: self.sort.clone(),
                    mode: self.mode.clone(),
                };
                let _ = save_config(&config_path, &config);
            }
        }
    }

    pub fn get_filtered_sorted_rows(&self) -> Vec<RegionQueueData> {
        let mut filtered: Vec<RegionQueueData> = self.queues.iter().filter(|r| {
            if self.mode == "standard" { r.mode == "Standard" }
            else if self.mode == "event" { r.mode == "Event" }
            else { r.mode == "Standard" } // Fallback
        }).cloned().collect();

        filtered.sort_by(|a, b| {
            let a_is_prio = self.priority.contains(&a.name);
            let b_is_prio = self.priority.contains(&b.name);
            
            if a_is_prio != b_is_prio {
                return b_is_prio.cmp(&a_is_prio); // true > false, so prioritized comes first
            }

            if self.sort == "survivor" {
                api::parse_time_to_seconds(&a.survivor).cmp(&api::parse_time_to_seconds(&b.survivor))
            } else if self.sort == "killer" {
                api::parse_time_to_seconds(&a.killer).cmp(&api::parse_time_to_seconds(&b.killer))
            } else {
                a.name.cmp(&b.name)
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
    let table_height = (rows_data.len() as u16) + 4; // 1 header, 1 margin, 2 borders

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
    let title_text = if is_ru { " Время ожидания Dead by Daylight " } else { " Dead by Daylight Queue Times " };
    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled(title_text, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Table

    let rows = rows_data.into_iter().map(|item| {
        let name_style = if app.priority.contains(&item.name) {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let surv_color = color_for_time(&item.survivor);
        let kill_color = color_for_time(&item.killer);

        let flag_spaced = if item.flag.is_empty() { String::new() } else { format!("{} ", item.flag) };
        let full_name = format!("{}{}", flag_spaced, item.name);
        
        Row::new(vec![
            Cell::from(Span::styled(full_name, name_style)),
            Cell::from(Span::styled(item.survivor.clone(), Style::default().fg(surv_color))),
            Cell::from(Span::styled(item.killer.clone(), Style::default().fg(kill_color))),
        ])
    });

    let active_sort_str = match app.sort.as_str() {
        "survivor" => if is_ru { "Выживший" } else { "Survivor" },
        "killer" => if is_ru { "Маньяк" } else { "Killer" },
        "priority" => if is_ru { "Приоритет" } else { "Priority" },
        _ => if is_ru { "По умолч." } else { "Default" },
    };

    let mode_str = if is_ru {
        match app.mode.as_str() {
            "standard" => "Обычный",
            "event" => "Ивент",
            other => other,
        }
    } else {
        app.mode.as_str()
    };

    let table_title = if is_ru {
        format!(" Сортировка: {} | Режим: {} ", active_sort_str, mode_str)
    } else {
        format!(" Sort: {} | Mode: {} ", active_sort_str, mode_str)
    };

    let hdr_region = if is_ru { "Регион" } else { "Region" };
    let hdr_survivor = if is_ru { "Выживший" } else { "Survivor" };
    let hdr_killer = if is_ru { "Маньяк" } else { "Killer" };

    let table = Table::new(rows, [
        Constraint::Length(35),
        Constraint::Length(12),
        Constraint::Length(12),
    ])
    .header(
        Row::new(vec![hdr_region, hdr_survivor, hdr_killer])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(1),
    )
    .block(Block::default().borders(Borders::ALL).title(table_title))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .column_spacing(1);

    f.render_stateful_widget(table, chunks[1], &mut app.table_state);

    // Footer
    let now = chrono::Utc::now().timestamp();
    let diff = now - app.api_last_updated;
    let time_str = if app.api_last_updated == 0 {
        if is_ru { "Загрузка...".to_string() } else { "Fetching...".to_string() }
    } else if diff < 60 {
        if is_ru { "только что".to_string() } else { "just now".to_string() }
    } else if diff < 3600 {
        if is_ru { format!("{} мин. назад", diff / 60) } else { format!("{} mins ago", diff / 60) }
    } else {
        if is_ru { format!("{} ч. {} мин. назад", diff / 3600, (diff % 3600) / 60) } else { format!("{} hrs {} mins ago", diff / 3600, (diff % 3600) / 60) }
    };

    let status_text = if let Some(ref err) = app.error_msg {
        if is_ru { format!("Ошибка: {}", err) } else { format!("Error: {}", err) }
    } else if app.is_fetching {
        if is_ru { "Обновление данных...".to_string() } else { "Updating data...".to_string() }
    } else {
        if is_ru { format!("Обновлено: {}", time_str) } else { format!("Updated: {}", time_str) }
    };

    let scroll_txt = if is_ru { "Прокрутка " } else { "Scroll " };
    let prio_txt = if is_ru { "Приоритет " } else { "Prioritize " };
    let quit_txt = if is_ru { "Выход " } else { "Quit " };
    let sort_txt = if is_ru { "Сортировка " } else { "Sort " };
    let mode_txt = if is_ru { "Режим " } else { "Toggle Mode " };

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" [\u{2191}\u{2193}] ", Style::default().fg(Color::Yellow)),
        Span::raw(scroll_txt),
        Span::styled(" [p] ", Style::default().fg(Color::Yellow)),
        Span::raw(prio_txt),
        Span::styled(" [q] ", Style::default().fg(Color::Yellow)),
        Span::raw(quit_txt),
        Span::styled(" [s/k/r] ", Style::default().fg(Color::Yellow)),
        Span::raw(sort_txt),
        Span::styled(" [m] ", Style::default().fg(Color::Yellow)),
        Span::raw(mode_txt),
        Span::raw(" | "),
        Span::styled(status_text, Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(footer, chunks[3]);
}

fn color_for_time(time_str: &str) -> Color {
    let sec = api::parse_time_to_seconds(time_str);
    if sec < 60 {
        Color::Green
    } else if sec < 180 {
        Color::Yellow
    } else {
        Color::Red
    }
}
