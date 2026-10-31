use crate::api;
use crate::app::{App, NoticeKind, SPINNER_FRAMES};
use crate::i18n::{TextKey, format_time_diff, tr, tr_mode, tr_sort, tr_time_format};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, ListState, Padding, Paragraph,
        Row, Table,
    },
};

pub const MIN_WIDTH: u16 = 48;
pub const MIN_HEIGHT: u16 = 12;

pub const STANDARD_WIDTH_BREAKPOINT: u16 = 80;
#[allow(dead_code)]
pub const NARROW_WIDTH_BREAKPOINT: u16 = 60;

pub const HEADER_HEIGHT: u16 = 1;
pub const FOOTER_HEIGHT: u16 = 1;
pub const EMPTY_TABLE_HEIGHT: u16 = 4;
pub const MODAL_WIDTH_PERCENT: u16 = 65;
pub const MODAL_HEIGHT_PERCENT: u16 = 80;
pub const TABLE_WIDTH: u16 = 69;
pub const NARROW_TABLE_WIDTH: u16 = 44;
pub const SUMMARY_HEIGHT: u16 = 4;
pub const SUMMARY_LABEL_WIDTH: usize = 11;
pub const SUMMARY_TIME_WIDTH: usize = 8;
pub const SUMMARY_REGION_WIDTH: usize = 18;
pub const SUMMARY_PING_WIDTH: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub border_normal: Color,
    pub border_focus: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub subtle_text: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub no_color: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}

impl Theme {
    #[must_use]
    pub fn new() -> Self {
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
        Self::from_no_color(no_color)
    }

    #[must_use]
    pub fn from_no_color(no_color: bool) -> Self {
        if no_color {
            Self::no_color()
        } else {
            Self::color()
        }
    }

    #[must_use]
    pub fn no_color() -> Self {
        Self {
            border_normal: Color::Reset,
            border_focus: Color::Reset,
            text_primary: Color::Reset,
            text_secondary: Color::Reset,
            subtle_text: Color::Reset,
            accent: Color::Reset,
            success: Color::Reset,
            warning: Color::Reset,
            error: Color::Reset,
            info: Color::Reset,
            no_color: true,
        }
    }

    #[must_use]
    pub fn color() -> Self {
        Self {
            border_normal: Color::DarkGray,
            border_focus: Color::LightRed,
            text_primary: Color::White,
            text_secondary: Color::Gray,
            subtle_text: Color::DarkGray,
            accent: Color::LightRed,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::LightYellow,
            no_color: false,
        }
    }

    pub fn color_for_queue_time(&self, time_str: &str) -> Color {
        if self.no_color || time_str == "—" || time_str.is_empty() {
            return self.subtle_text;
        }
        let sec = api::parse_time_to_seconds(time_str);
        if sec < 60 {
            self.success
        } else if sec < 180 {
            self.warning
        } else {
            self.error
        }
    }

    pub fn color_for_ping(&self, ms: Option<u32>) -> Color {
        if self.no_color {
            return self.subtle_text;
        }
        crate::ping::color_for_ping(ms)
    }

    pub fn primary_style(&self) -> Style {
        Style::default()
            .fg(self.text_primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn subtle_style(&self) -> Style {
        Style::default().fg(self.subtle_text)
    }

    pub fn accent_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }
}

#[allow(dead_code)]
pub fn color_for_time(time_str: &str) -> Color {
    Theme::new().color_for_queue_time(time_str)
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

fn pad_right(s: &str, width: usize) -> String {
    let pad = width.saturating_sub(s.chars().count());
    format!("{}{}", s, " ".repeat(pad))
}

pub fn draw_floor_warning(f: &mut Frame, theme: &Theme, area: Rect) {
    let msg = vec![
        Line::from(Span::styled("Terminal too small", theme.accent_style())),
        Line::from(Span::styled(
            format!("Current size: {}x{}", area.width, area.height),
            theme.subtle_style(),
        )),
        Line::from(Span::styled(
            format!("Minimum required: {}x{}", MIN_WIDTH, MIN_HEIGHT),
            theme.primary_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Please resize your window or press 'q' to quit",
            theme.subtle_style(),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme.subtle_style());
    let paragraph = Paragraph::new(msg)
        .block(block)
        .alignment(Alignment::Center);
    f.render_widget(paragraph, area);
}

pub fn draw_header(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let active_sort_str = tr_sort(app.locale, app.sort);
    let mode_str = tr_mode(app.locale, app.mode);
    let lock_str = if app.locked.is_empty() {
        tr(app.locale, TextKey::LockNone)
    } else {
        tr(app.locale, TextKey::LockActive)
    };

    let sort_indicator = match app.sort {
        crate::config::SortOrder::Killer
        | crate::config::SortOrder::Survivor
        | crate::config::SortOrder::Ping => " ▲",
        crate::config::SortOrder::Default => "",
    };

    let mut spans = vec![
        Span::styled(tr(app.locale, TextKey::SortLabel), theme.subtle_style()),
        Span::styled(
            format!("{}{}", active_sort_str, sort_indicator),
            theme.primary_style(),
        ),
        Span::styled("  │  ", theme.subtle_style()),
        Span::styled(tr(app.locale, TextKey::ModeLabel), theme.subtle_style()),
        Span::styled(mode_str, theme.primary_style()),
    ];

    if app.mode == crate::config::GameMode::Event {
        let now_ts = chrono::Local::now().timestamp();
        if let Some(ref ev) = app.current_event
            && let Ok(end_ts) = ev.end.trim().parse::<i64>()
        {
            let remaining = (end_ts - now_ts).max(0) as u64;
            let countdown = crate::api::format_event_countdown(remaining, app.time_format);
            let left_str = tr(app.locale, TextKey::EventLeft);
            let text = format!(" [{} - {} {}]", ev.name, countdown, left_str);
            spans.push(Span::styled(text, Style::default().fg(theme.info)));
        } else if let Some(ref ev) = app.upcoming_event
            && let Some(dt) = crate::api::event_timestamp_to_local(&ev.start)
        {
            let dt_str = crate::api::format_local_datetime(dt);
            let upcoming_str = tr(app.locale, TextKey::EventUpcoming);
            let starts_str = tr(app.locale, TextKey::EventStartsIn);
            let text = format!(
                " [{}: {} - {} {}]",
                upcoming_str, ev.name, starts_str, dt_str
            );
            spans.push(Span::styled(text, theme.subtle_style()));
        }
    }

    spans.extend([
        Span::styled("  │  ", theme.subtle_style()),
        Span::styled(tr(app.locale, TextKey::TimeLabel), theme.subtle_style()),
        Span::styled(
            tr_time_format(app.locale, app.time_format),
            theme.primary_style(),
        ),
        Span::styled("  │  ", theme.subtle_style()),
        Span::styled(tr(app.locale, TextKey::LockLabel), theme.subtle_style()),
        Span::styled(
            lock_str,
            if !app.locked.is_empty() {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme.subtle_style()
            },
        ),
    ]);

    let header = Paragraph::new(Line::from(spans));
    f.render_widget(header, area);
}

pub fn draw_table(f: &mut Frame, app: &mut App, theme: &Theme, area: Rect, is_narrow: bool) {
    let rows_data = app.get_filtered_sorted_rows();
    let is_empty = rows_data.is_empty();

    let api_to_aws = api::get_api_to_aws();
    let (rows, col_constraints, spacing) = if is_empty {
        let (msg, color) = if app.is_fetching {
            (
                tr(app.locale, TextKey::FetchingQueues).to_string(),
                theme.accent,
            )
        } else if app
            .notice
            .as_ref()
            .is_some_and(|n| n.kind == NoticeKind::Error)
        {
            (
                tr(app.locale, TextKey::FailedQueues).to_string(),
                theme.error,
            )
        } else if app.mode == crate::config::GameMode::Event {
            if let Some(ref ev) = app.upcoming_event {
                if let Some(dt) = crate::api::event_timestamp_to_local(&ev.start) {
                    let dt_str = crate::api::format_local_datetime(dt);
                    let now_ts = chrono::Local::now().timestamp();
                    let start_ts = ev.start.trim().parse::<i64>().unwrap_or(0);
                    let remaining = (start_ts - now_ts).max(0) as u64;
                    let countdown = crate::api::format_event_countdown(remaining, app.time_format);
                    let upcoming_str = tr(app.locale, TextKey::EventUpcoming);
                    let starts_str = tr(app.locale, TextKey::EventStartsIn);
                    let in_str = tr(app.locale, TextKey::EventIn);
                    (
                        format!(
                            "  {}: {} ({} {} - {} {})",
                            upcoming_str, ev.name, starts_str, dt_str, in_str, countdown
                        ),
                        theme.info,
                    )
                } else {
                    (
                        format!("  {}", tr(app.locale, TextKey::NoDataForMode)),
                        theme.subtle_text,
                    )
                }
            } else {
                (
                    format!("  {}", tr(app.locale, TextKey::NoDataForMode)),
                    theme.subtle_text,
                )
            }
        } else {
            (
                tr(app.locale, TextKey::NoDataForMode).to_string(),
                theme.subtle_text,
            )
        };
        (
            vec![Row::new(vec![
                Cell::from(Span::styled(msg, Style::default().fg(color))),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
            ])],
            vec![
                Constraint::Percentage(100),
                Constraint::Length(0),
                Constraint::Length(0),
                Constraint::Length(0),
            ],
            1,
        )
    } else {
        let r: Vec<Row> = rows_data
            .into_iter()
            .map(|item| {
                let aws_code = api_to_aws.get(item.name.as_str()).unwrap_or(&"");
                let is_whitelisted = app.locked.contains(*aws_code);
                let is_disabled = item.is_disabled();

                let reg_str = if is_narrow || item.flag.is_empty() {
                    item.name.clone()
                } else {
                    format!("{} {}", item.flag, item.name)
                };

                if is_disabled {
                    let dim_style = theme.subtle_style();
                    Row::new(vec![
                        Cell::from(Span::styled(reg_str, dim_style)),
                        Cell::from(
                            Line::from(Span::styled("—", dim_style)).alignment(Alignment::Right),
                        ),
                        Cell::from(
                            Line::from(Span::styled("—", dim_style)).alignment(Alignment::Right),
                        ),
                        Cell::from(
                            Line::from(Span::styled("—", dim_style)).alignment(Alignment::Right),
                        ),
                    ])
                } else {
                    let name_style = if is_whitelisted {
                        theme.accent_style()
                    } else {
                        Style::default()
                    };

                    let (ping_str, ping_color) = if let Some(&ms) = app.pings.get(*aws_code) {
                        (format!("{} ms", ms), theme.color_for_ping(Some(ms)))
                    } else if app.pings.is_empty() {
                        ("...".to_string(), theme.subtle_text)
                    } else {
                        ("—".to_string(), theme.subtle_text)
                    };

                    let surv_color = theme.color_for_queue_time(&item.survivor);
                    let kill_color = theme.color_for_queue_time(&item.killer);

                    Row::new(vec![
                        Cell::from(Span::styled(reg_str, name_style)),
                        Cell::from(
                            Line::from(Span::styled(ping_str, Style::default().fg(ping_color)))
                                .alignment(Alignment::Right),
                        ),
                        Cell::from(
                            Line::from(Span::styled(
                                item.survivor.clone(),
                                Style::default().fg(surv_color),
                            ))
                            .alignment(Alignment::Right),
                        ),
                        Cell::from(
                            Line::from(Span::styled(
                                item.killer.clone(),
                                Style::default().fg(kill_color),
                            ))
                            .alignment(Alignment::Right),
                        ),
                    ])
                }
            })
            .collect();

        let (constraints, spacing) = if is_narrow {
            (
                vec![
                    Constraint::Length(15),
                    Constraint::Length(7),
                    Constraint::Length(9),
                    Constraint::Length(10),
                ],
                1,
            )
        } else {
            (
                vec![
                    Constraint::Length(24),
                    Constraint::Length(11),
                    Constraint::Length(14),
                    Constraint::Length(14),
                ],
                2,
            )
        };

        (r, constraints, spacing)
    };

    let ping_indicator = if app.sort == crate::config::SortOrder::Ping {
        " ▲"
    } else {
        ""
    };
    let surv_indicator = if app.sort == crate::config::SortOrder::Survivor {
        " ▲"
    } else {
        ""
    };
    let kill_indicator = if app.sort == crate::config::SortOrder::Killer {
        " ▲"
    } else {
        ""
    };

    let (hdr_region, hdr_ping, hdr_survivor, hdr_killer) = if is_narrow {
        (
            "Region".to_string(),
            format!("Ping{}", ping_indicator),
            format!("Surv{}", surv_indicator),
            format!("Kill{}", kill_indicator),
        )
    } else {
        (
            tr(app.locale, TextKey::ColRegion).to_string(),
            format!("{}{}", tr(app.locale, TextKey::ColPing), ping_indicator),
            format!("{}{}", tr(app.locale, TextKey::ColSurvivor), surv_indicator),
            format!("{}{}", tr(app.locale, TextKey::ColKiller), kill_indicator),
        )
    };

    let header_row = Row::new(vec![
        Cell::from(Span::styled(hdr_region, theme.primary_style())),
        Cell::from(
            Line::from(Span::styled(hdr_ping, theme.primary_style())).alignment(Alignment::Right),
        ),
        Cell::from(
            Line::from(Span::styled(hdr_survivor, theme.primary_style()))
                .alignment(Alignment::Right),
        ),
        Cell::from(
            Line::from(Span::styled(hdr_killer, theme.primary_style())).alignment(Alignment::Right),
        ),
    ])
    .bottom_margin(1);

    let table_width = if is_narrow {
        NARROW_TABLE_WIDTH
    } else {
        TABLE_WIDTH
    };
    let table_area = Rect {
        x: area.x,
        y: area.y,
        width: table_width.min(area.width),
        height: area.height,
    };

    let mut table = Table::new(rows, col_constraints)
        .header(header_row)
        .column_spacing(spacing);

    if !is_empty {
        table = table.row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    }

    f.render_stateful_widget(table, table_area, &mut app.table_state);
}

pub fn draw_footer(f: &mut Frame, app: &App, theme: &Theme, area: Rect, is_narrow: bool) {
    let now = chrono::Utc::now().timestamp();
    let diff = now - app.api_last_updated;
    let time_str = if app.api_last_updated == 0 {
        tr(app.locale, TextKey::TimeFetching).to_string()
    } else {
        format_time_diff(app.locale, diff)
    };

    let status_span = if let Some(ref notice) = app.notice {
        let (prefix, color) = match notice.kind {
            NoticeKind::Error => ("✗ ", theme.error),
            NoticeKind::Info => ("ℹ ", theme.accent),
            NoticeKind::Success => ("✓ ", theme.success),
        };
        Span::styled(
            format!("{}{}", prefix, notice.message),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )
    } else if app.is_fetching {
        let spinner = SPINNER_FRAMES[app.spinner_frame % SPINNER_FRAMES.len()];
        let msg = tr(app.locale, TextKey::StatusFetching);
        Span::styled(format!("{} {}", spinner, msg), theme.accent_style())
    } else {
        let prefix = tr(app.locale, TextKey::StatusApiUpdated);
        Span::styled(format!("{}{}", prefix, time_str), theme.subtle_style())
    };

    let mut spans = vec![status_span, Span::styled("  │  ", theme.subtle_style())];

    let select_txt = tr(app.locale, TextKey::ActionSelect).trim();
    let lock_txt = tr(app.locale, TextKey::ActionLock).trim();
    let sort_txt = tr(app.locale, TextKey::ActionSort).trim();
    let mode_txt = tr(app.locale, TextKey::ActionMode).trim();
    let time_txt = tr(app.locale, TextKey::ActionTime).trim();
    let refresh_txt = tr(app.locale, TextKey::ActionRefresh).trim();
    let quit_txt = tr(app.locale, TextKey::ActionQuit).trim();

    if area.width < 50 {
        spans.extend([
            Span::styled("↑↓", theme.accent_style()),
            Span::styled(" nav ", theme.subtle_style()),
            Span::styled("L", theme.accent_style()),
            Span::styled(" lock ", theme.subtle_style()),
            Span::styled("q", theme.accent_style()),
            Span::styled(" quit", theme.subtle_style()),
        ]);
    } else if is_narrow {
        spans.extend([
            Span::styled("↑↓", theme.accent_style()),
            Span::styled(" nav  ", theme.subtle_style()),
            Span::styled("L", theme.accent_style()),
            Span::styled(" lock  ", theme.subtle_style()),
            Span::styled("S", theme.accent_style()),
            Span::styled(" sort  ", theme.subtle_style()),
            Span::styled("M", theme.accent_style()),
            Span::styled(" mode  ", theme.subtle_style()),
            Span::styled("q", theme.accent_style()),
            Span::styled(" quit", theme.subtle_style()),
        ]);
    } else if area.width < 96 {
        spans.extend([
            Span::styled("↑↓ ", theme.accent_style()),
            Span::styled(format!("{} · ", select_txt), theme.subtle_style()),
            Span::styled("L ", theme.accent_style()),
            Span::styled(format!("{} · ", lock_txt), theme.subtle_style()),
            Span::styled("S ", theme.accent_style()),
            Span::styled(format!("{} · ", sort_txt), theme.subtle_style()),
            Span::styled("M ", theme.accent_style()),
            Span::styled(format!("{} · ", mode_txt), theme.subtle_style()),
            Span::styled("T ", theme.accent_style()),
            Span::styled(format!("{} · ", time_txt), theme.subtle_style()),
            Span::styled("q ", theme.accent_style()),
            Span::styled(quit_txt, theme.subtle_style()),
        ]);
    } else {
        spans.extend([
            Span::styled("↑↓/jk ", theme.accent_style()),
            Span::styled(format!("{} · ", select_txt), theme.subtle_style()),
            Span::styled("L ", theme.accent_style()),
            Span::styled(format!("{} · ", lock_txt), theme.subtle_style()),
            Span::styled("S ", theme.accent_style()),
            Span::styled(format!("{} · ", sort_txt), theme.subtle_style()),
            Span::styled("M ", theme.accent_style()),
            Span::styled(format!("{} · ", mode_txt), theme.subtle_style()),
            Span::styled("T ", theme.accent_style()),
            Span::styled(format!("{} · ", time_txt), theme.subtle_style()),
            Span::styled("R ", theme.accent_style()),
            Span::styled(format!("{} · ", refresh_txt), theme.subtle_style()),
            Span::styled("Esc/q ", theme.accent_style()),
            Span::styled(quit_txt, theme.subtle_style()),
        ]);
    }

    let footer = Paragraph::new(Line::from(spans));
    f.render_widget(footer, area);
}

pub fn draw_lock_modal(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    f.render_widget(Clear, area);

    let modal_title = tr(app.locale, TextKey::ModalTitle);
    let toggle_txt = tr(app.locale, TextKey::ModalToggle);
    let all_txt = tr(app.locale, TextKey::ModalAll);
    let invert_txt = tr(app.locale, TextKey::ModalInvert);
    let apply_txt = tr(app.locale, TextKey::ModalApply);
    let cancel_txt = tr(app.locale, TextKey::ModalCancel);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.accent_style())
        .title(Span::styled(
            format!(" {} ", modal_title),
            theme.primary_style(),
        ))
        .padding(Padding::horizontal(1));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner_area);

    let modal_regions = app.get_modal_regions();
    let aws_to_api = api::get_aws_to_api();
    let aws_to_flag = api::get_aws_to_flag();

    let items: Vec<ListItem> = modal_regions
        .iter()
        .enumerate()
        .map(|(idx, code)| {
            let name = aws_to_api.get(*code).unwrap_or(code);
            let flag = aws_to_flag.get(*code).unwrap_or(&"");
            let flag_str = if flag.is_empty() {
                String::new()
            } else {
                format!("{} ", flag)
            };
            let is_selected = app.lock_modal_selected.iter().any(|r| r == *code);

            let checkbox = if is_selected { "[*] " } else { "[ ] " };

            let ping_str = if let Some(&ms) = app.pings.get(*code) {
                format!(" - {} ms", ms)
            } else {
                String::new()
            };

            let text = format!("{}{}{} ({}){}", checkbox, flag_str, name, code, ping_str);

            let mut style = if is_selected {
                Style::default().fg(theme.success)
            } else {
                theme.subtle_style()
            };

            if idx == app.lock_modal_cursor {
                style = style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
            }

            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items);
    let mut list_state = ListState::default();
    list_state.select(Some(app.lock_modal_cursor));
    f.render_stateful_widget(list, modal_chunks[0], &mut list_state);

    let modal_instructions = Line::from(vec![
        Span::styled(" [Space] ", theme.accent_style()),
        Span::styled(toggle_txt, Style::default().fg(Color::Reset)),
        Span::styled("  [a] ", theme.accent_style()),
        Span::styled(all_txt, Style::default().fg(Color::Reset)),
        Span::styled("  [i] ", theme.accent_style()),
        Span::styled(invert_txt, Style::default().fg(Color::Reset)),
        Span::styled("  [Enter] ", theme.accent_style()),
        Span::styled(apply_txt, Style::default().fg(Color::Reset)),
        Span::styled("  [Esc] ", theme.accent_style()),
        Span::styled(cancel_txt, Style::default().fg(Color::Reset)),
    ]);
    let footer_widget = Paragraph::new(modal_instructions);
    f.render_widget(footer_widget, modal_chunks[1]);
}

pub fn draw_summary(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if area.height < SUMMARY_HEIGHT {
        return;
    }

    let summary_area = Rect {
        x: area.x,
        y: area.y,
        width: TABLE_WIDTH.min(area.width),
        height: SUMMARY_HEIGHT,
    };

    let summary = app.summary();
    let api_to_aws = api::get_api_to_aws();

    let format_pick_line = |key: TextKey,
                            pick: Option<&crate::app::BestPick<'_>>,
                            role: crate::app::Role|
     -> Line<'_> {
        let label_span = Span::styled(
            pad_right(tr(app.locale, key), SUMMARY_LABEL_WIDTH),
            theme.subtle_style(),
        );
        if let Some(best) = pick {
            let time_str = role.queue_time(best.row);
            let time_color = theme.color_for_queue_time(time_str);
            let time_span = Span::styled(
                pad_right(time_str, SUMMARY_TIME_WIDTH),
                Style::default().fg(time_color),
            );

            let reg_str = if best.row.flag.is_empty() {
                best.row.name.clone()
            } else {
                format!("{} {}", best.row.flag, best.row.name)
            };
            let aws_code = api_to_aws.get(best.row.name.as_str()).unwrap_or(&"");
            let is_locked = app.locked.contains(*aws_code);
            let name_style = if is_locked {
                theme.accent_style()
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };
            let name_span = Span::styled(pad_right(&reg_str, SUMMARY_REGION_WIDTH), name_style);

            let ping = app.pings.get(*aws_code).copied();
            let (ping_text, ping_color) = if let Some(ms) = ping {
                (format!("{} ms", ms), theme.color_for_ping(Some(ms)))
            } else {
                ("—".to_string(), theme.subtle_text)
            };
            let ping_span = Span::styled(
                pad_right(&ping_text, SUMMARY_PING_WIDTH),
                Style::default().fg(ping_color),
            );

            let mut spans = vec![label_span, time_span, name_span, ping_span];

            if best.similar > 0 {
                let sim_str = format!(
                    "+{} {}",
                    best.similar,
                    tr(app.locale, TextKey::SummarySimilar)
                );
                spans.push(Span::styled(sim_str, theme.subtle_style()));
            }

            Line::from(spans)
        } else {
            Line::from(vec![label_span, Span::styled("—", theme.subtle_style())])
        }
    };

    let killer_line = format_pick_line(
        TextKey::SummaryKiller,
        summary.killer.as_ref(),
        crate::app::Role::Killer,
    );
    let survivor_line = format_pick_line(
        TextKey::SummarySurvivor,
        summary.survivor.as_ref(),
        crate::app::Role::Survivor,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme.subtle_style())
        .title(Span::styled(
            format!(" {} ", tr(app.locale, TextKey::SummaryTitle)),
            theme.accent_style(),
        ))
        .padding(Padding::horizontal(1));

    let paragraph = Paragraph::new(vec![killer_line, survivor_line]).block(block);
    f.render_widget(paragraph, summary_area);
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let theme = Theme::new();

    // 1. Minimum Floor Guard (< 48 cols or < 12 rows)
    if f.area().width < MIN_WIDTH || f.area().height < MIN_HEIGHT {
        draw_floor_warning(f, &theme, f.area());
        return;
    }

    let is_narrow = f.area().width < STANDARD_WIDTH_BREAKPOINT;
    let h_pad = if is_narrow { 1 } else { 2 };

    // 2. Single outer border around the main application with internal horizontal padding
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme.subtle_style())
        .title(Span::styled(
            format!(" {} ", tr(app.locale, TextKey::HeaderTitle)),
            theme.accent_style(),
        ))
        .padding(Padding::horizontal(h_pad));
    let inner_area = outer_block.inner(f.area());
    f.render_widget(outer_block, f.area());

    // 3. Layout partitioning inside outer frame
    let rows_data = app.get_filtered_sorted_rows();
    let is_empty = rows_data.is_empty();
    let desired_table_height = if is_empty {
        EMPTY_TABLE_HEIGHT
    } else {
        (rows_data.len() as u16) + 2 // header row + bottom margin
    };

    let non_table_fixed = HEADER_HEIGHT + SUMMARY_HEIGHT + FOOTER_HEIGHT;
    let (show_summary, table_height) = if inner_area.height >= non_table_fixed + 4 && !is_narrow {
        let available_for_table = inner_area.height.saturating_sub(non_table_fixed);
        (true, desired_table_height.min(available_for_table))
    } else if inner_area.height >= HEADER_HEIGHT + FOOTER_HEIGHT + 4 {
        let available_without_summary = inner_area
            .height
            .saturating_sub(HEADER_HEIGHT + FOOTER_HEIGHT);
        (false, desired_table_height.min(available_without_summary))
    } else {
        (false, desired_table_height.min(inner_area.height))
    };

    let mut constraints = vec![
        Constraint::Length(HEADER_HEIGHT), // 0: Header strip
    ];
    if show_summary {
        constraints.push(Constraint::Length(SUMMARY_HEIGHT)); // 1: Hero Best Pick card
    }
    constraints.push(Constraint::Min(table_height)); // Table
    constraints.push(Constraint::Length(FOOTER_HEIGHT)); // Footer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    if show_summary {
        draw_header(f, app, &theme, chunks[0]);
        draw_summary(f, app, &theme, chunks[1]);
        draw_table(f, app, &theme, chunks[2], is_narrow);
        draw_footer(f, app, &theme, chunks[3], is_narrow);
    } else {
        draw_header(f, app, &theme, chunks[0]);
        draw_table(f, app, &theme, chunks[1], is_narrow);
        draw_footer(f, app, &theme, chunks[2], is_narrow);
    }

    // 4. Modal overlay (if active)
    if app.show_lock_modal {
        let modal_area = centered_rect(MODAL_WIDTH_PERCENT, MODAL_HEIGHT_PERCENT, f.area());
        draw_lock_modal(f, app, &theme, modal_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GameMode, Language, SortOrder, TimeFormat};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_modal_label_color_via_test_backend() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Exact,
            vec![],
            Language::En,
            None,
        );
        app.show_lock_modal = true;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| draw(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();

        // Search the buffer for the modal action line containing "[Space]" and "Toggle"
        let mut found_line = false;
        for y in 0..buffer.area.height {
            let line_chars: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();

            if line_chars.contains("[Space]") && line_chars.contains("Toggle") {
                found_line = true;
                // Find column index of '[' in "[Space]"
                let byte_bracket = line_chars.find('[').unwrap();
                let bracket_x = line_chars[..byte_bracket].chars().count() as u16;
                let bracket_cell = &buffer[(bracket_x, y)];
                assert_eq!(
                    bracket_cell.fg,
                    Color::LightRed,
                    "Bracket cell '[' should be LightRed"
                );

                // Find column index of 'T' in "Toggle"
                let byte_toggle = line_chars.find("Toggle").unwrap();
                let toggle_x = line_chars[..byte_toggle].chars().count() as u16;
                let text_cell = &buffer[(toggle_x, y)];
                assert_ne!(
                    text_cell.fg,
                    Color::LightRed,
                    "Label text 'Toggle' must not inherit LightRed border color"
                );
                assert_eq!(
                    text_cell.fg,
                    Color::Reset,
                    "Label text 'Toggle' should be default/Reset fg"
                );
                break;
            }
        }

        assert!(
            found_line,
            "Modal action instruction line not found in buffer"
        );
    }

    #[test]
    fn test_draw_does_not_mutate_state() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Exact,
            vec!["eu-central-1".to_string()],
            Language::En,
            None,
        );
        app.queues = vec![
            api::RegionQueueData::new("[DE]", "Frankfurt", "Standard", "10s", "6s"),
            api::RegionQueueData::new("[IE]", "Dublin", "Standard", "12s", "8s"),
        ];
        app.pings.insert("eu-central-1".to_string(), 35);
        app.pings.insert("eu-west-1".to_string(), 45);
        app.table_state.select(Some(0));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| draw(f, &mut app)).unwrap();

        // Ensure table selection was not cleared by draw()
        assert_eq!(app.table_state.selected(), Some(0));

        // Verify summary panel was drawn and exercised
        let buffer = terminal.backend().buffer();
        let mut found_summary = false;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Best Pick") {
                found_summary = true;
                break;
            }
        }
        assert!(found_summary, "Summary panel should be rendered in buffer");
    }

    #[test]
    fn test_draw_with_full_regions_on_80x24_terminal() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Exact,
            vec![],
            Language::En,
            None,
        );
        let all_aws = api::get_all_aws_regions();
        let aws_to_api = api::get_aws_to_api();
        for &code in &all_aws {
            let name = aws_to_api.get(code).unwrap_or(&code);
            app.queues.push(api::RegionQueueData::new(
                "", *name, "Standard", "10s", "15s",
            ));
            app.pings.insert(code.to_string(), 40);
        }
        assert_eq!(app.queues.len(), 15);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| draw(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let mut found_summary = false;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Best Pick") {
                found_summary = true;
                break;
            }
        }
        assert!(
            found_summary,
            "Summary panel must remain visible on 80x24 terminal even with full 15 regions"
        );
    }

    #[test]
    fn test_draw_summary_russian_and_skip_height() {
        let theme = Theme::new();
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Exact,
            vec!["eu-central-1".to_string()],
            Language::Ru,
            None,
        );
        app.queues = vec![
            api::RegionQueueData::new("[DE]", "Frankfurt", "Standard", "10s", "6s"),
            api::RegionQueueData::new("[IE]", "Dublin", "Standard", "10s", "6s"),
        ];
        app.pings.insert("eu-central-1".to_string(), 35);
        app.pings.insert("eu-west-1".to_string(), 45);

        // Test skip rendering when height < 4
        let backend_small = TestBackend::new(80, 3);
        let mut term_small = Terminal::new(backend_small).unwrap();
        term_small
            .draw(|f| draw_summary(f, &app, &theme, Rect::new(0, 0, 80, 3)))
            .unwrap();
        let buf_small = term_small.backend().buffer();
        for y in 0..3 {
            let line: String = (0..80)
                .map(|x| buf_small[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            assert!(!line.contains("Лучший выбор"));
        }

        // Test Russian rendering when height >= 4
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| draw_summary(f, &app, &theme, Rect::new(0, 0, 80, 10)))
            .unwrap();
        let buf = terminal.backend().buffer();
        let mut found_title = false;
        let mut found_killer = false;
        let mut found_similar = false;
        for y in 0..10 {
            let line: String = (0..80)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Лучший выбор") {
                found_title = true;
            }
            if line.contains("Маньяк:") {
                found_killer = true;
            }
            if line.contains("+1 похожих") {
                found_similar = true;
            }
        }
        assert!(found_title, "Should contain Russian summary title");
        assert!(found_killer, "Should contain Russian killer label");
        assert!(found_similar, "Should contain Russian similar suffix");

        // Verify that rows y >= 4 are completely empty (panel height tightly constrained to 4)
        for y in 4..10 {
            let line: String = (0..80)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            assert!(
                line.trim().is_empty(),
                "Line {} should be empty outside summary panel, got: '{}'",
                y,
                line
            );
        }
    }

    #[test]
    fn test_draw_summary_alignment_and_tight_height() {
        let theme = Theme::new();
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Exact,
            vec!["eu-central-1".to_string()],
            Language::En,
            None,
        );
        app.queues = vec![
            api::RegionQueueData::new("[DE]", "Frankfurt", "Standard", "12s", "6s"),
            api::RegionQueueData::new("[IE]", "Dublin", "Standard", "12s", "6s"),
        ];
        app.pings.insert("eu-central-1".to_string(), 62);
        app.pings.insert("eu-west-1".to_string(), 70);

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| draw_summary(f, &app, &theme, Rect::new(0, 0, 80, 10)))
            .unwrap();
        let buf = terminal.backend().buffer();

        let l0: String = (0..80)
            .map(|x| buf[(x, 0)].symbol().chars().next().unwrap_or(' '))
            .collect();
        let l1: String = (0..80)
            .map(|x| buf[(x, 1)].symbol().chars().next().unwrap_or(' '))
            .collect();
        let l2: String = (0..80)
            .map(|x| buf[(x, 2)].symbol().chars().next().unwrap_or(' '))
            .collect();
        let l3: String = (0..80)
            .map(|x| buf[(x, 3)].symbol().chars().next().unwrap_or(' '))
            .collect();

        // Check top border and title
        assert!(l0.contains("Best Pick"), "Line 0 should have title");
        assert!(l0.contains('┌'), "Line 0 should have top border");

        // Check rows: Killer and Survivor present, Ping row deleted
        assert!(l1.contains("Killer:"), "Line 1 should be killer row");
        assert!(l2.contains("Survivor:"), "Line 2 should be survivor row");
        assert!(
            !l1.contains("Ping:") && !l2.contains("Ping:") && !l3.contains("Ping:"),
            "Ping row must be deleted"
        );

        // Check bottom border at line 3
        assert!(l3.contains('└'), "Line 3 should have bottom-left border");
        assert!(l3.contains('┘'), "Line 3 should have bottom-right border");

        // Check height tightening: lines 4..10 are empty
        for y in 4..10 {
            let line: String = (0..80)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            assert!(
                line.trim().is_empty(),
                "Line {} below height 4 must be empty, got: '{}'",
                y,
                line
            );
        }

        // Check clean ping formatting: no parentheses around ping
        assert!(
            !l1.contains("(62 ms)") && !l1.contains('('),
            "No parens around ping in killer row"
        );
        assert!(
            !l2.contains("(62 ms)") && !l2.contains('('),
            "No parens around ping in survivor row"
        );
        assert!(l1.contains("62 ms"), "Killer row should display 62 ms");
        assert!(l2.contains("62 ms"), "Survivor row should display 62 ms");

        // Check column alignment: region and ping must align vertically across Killer and Survivor
        let reg_pos_1 = l1.find("[DE] Frankfurt").expect("Region in line 1");
        let reg_pos_2 = l2.find("[DE] Frankfurt").expect("Region in line 2");
        assert_eq!(
            reg_pos_1, reg_pos_2,
            "Region column should align between Killer and Survivor"
        );

        let ping_pos_1 = l1.find("62 ms").expect("Ping in line 1");
        let ping_pos_2 = l2.find("62 ms").expect("Ping in line 2");
        assert_eq!(
            ping_pos_1, ping_pos_2,
            "Ping column should align between Killer and Survivor"
        );
    }

    #[test]
    fn test_header_draws_time_format() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Exact,
            vec![],
            Language::En,
            None,
        );

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        let mut found_time = false;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Time: ") && line.contains("Exact") {
                found_time = true;
                break;
            }
        }
        assert!(found_time, "Header must display 'Time: Exact'");

        // Toggle to rounded
        app.toggle_time_format();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer2 = terminal.backend().buffer();

        let mut found_rounded = false;
        for y in 0..buffer2.area.height {
            let line: String = (0..buffer2.area.width)
                .map(|x| buffer2[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Time: ") && line.contains("~Rounded") {
                found_rounded = true;
                break;
            }
        }
        assert!(found_rounded, "Header must display 'Time: ~Rounded'");
    }

    #[test]
    fn test_header_event_badge_only_in_event_mode() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Rounded,
            vec![],
            Language::En,
            None,
        );
        app.current_event = Some(api::EventItem {
            name: "2v8 event".to_string(),
            start: "1000".to_string(),
            end: format!("{}", chrono::Local::now().timestamp() + 86400 * 11),
        });

        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. Standard mode: event badge must NOT appear
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();
        let mut standard_badge_found = false;
        let mut standard_mode_found = false;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Mode: Standard") {
                standard_mode_found = true;
            }
            if line.contains("2v8 event") {
                standard_badge_found = true;
            }
        }
        assert!(standard_mode_found);
        assert!(
            !standard_badge_found,
            "Event badge must not appear in Standard mode"
        );

        // 2. Event mode: event badge appears right after Mode: Event
        app.mode = GameMode::Event;
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer2 = terminal.backend().buffer();
        let mut event_badge_found = false;
        for y in 0..buffer2.area.height {
            let line: String = (0..buffer2.area.width)
                .map(|x| buffer2[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Mode: Event [2v8 event - 11d left]") {
                event_badge_found = true;
                break;
            }
        }
        assert!(event_badge_found, "Event badge must appear in Event mode");

        // 3. Rounding check: 10d 23h should round to 11d instead of truncating to 10d
        app.current_event = Some(api::EventItem {
            name: "2v8 event".to_string(),
            start: "1000".to_string(),
            end: format!(
                "{}",
                chrono::Local::now().timestamp() + 10 * 86400 + 23 * 3600
            ),
        });
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer3 = terminal.backend().buffer();
        let mut rounded_badge_found = false;
        for y in 0..buffer3.area.height {
            let line: String = (0..buffer3.area.width)
                .map(|x| buffer3[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Mode: Event [2v8 event - 11d left]") {
                rounded_badge_found = true;
                break;
            }
        }
        assert!(
            rounded_badge_found,
            "Event badge must correctly round 10d 23h to 11d"
        );
    }

    #[test]
    fn test_terminal_floor_under_48x12() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Exact,
            vec![],
            Language::En,
            None,
        );

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        let mut found_floor_msg = false;
        let mut found_min_size = false;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Terminal too small") {
                found_floor_msg = true;
            }
            if line.contains("Minimum required: 48x12") {
                found_min_size = true;
            }
        }
        assert!(
            found_floor_msg,
            "Must display 'Terminal too small' on small viewport"
        );
        assert!(found_min_size, "Must display 'Minimum required: 48x12'");
    }

    #[test]
    fn test_narrow_layout_60x24() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Exact,
            vec![],
            Language::En,
            None,
        );
        app.queues = vec![
            api::RegionQueueData::new("[DE]", "Frankfurt", "Standard", "10s", "6s"),
            api::RegionQueueData::new("[US]", "Virginia", "Standard", "15s", "8s"),
        ];

        let backend = TestBackend::new(60, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        let mut found_narrow_headers = false;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Region")
                && line.contains("Ping")
                && line.contains("Surv")
                && line.contains("Kill")
            {
                found_narrow_headers = true;
                break;
            }
        }
        assert!(
            found_narrow_headers,
            "Narrow viewport should render abbreviated headers (Region, Ping, Surv, Kill)"
        );
    }

    #[test]
    fn test_no_color_theme() {
        let theme_no_color = Theme::from_no_color(true);
        assert!(theme_no_color.no_color);
        assert_eq!(theme_no_color.border_normal, Color::Reset);
        assert_eq!(theme_no_color.accent, Color::Reset);
        assert_eq!(theme_no_color.color_for_queue_time("10s"), Color::Reset);
        assert_eq!(theme_no_color.color_for_ping(Some(30)), Color::Reset);
        assert_eq!(Theme::no_color(), theme_no_color);

        let theme_color = Theme::from_no_color(false);
        assert!(!theme_color.no_color);
        assert_ne!(theme_color.accent, Color::Reset);
        assert_eq!(theme_color.color_for_queue_time("10s"), Color::Green);
        assert_eq!(Theme::color(), theme_color);

        let default_theme = Theme::new();
        if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            assert_eq!(default_theme, Theme::no_color());
        } else {
            assert_eq!(default_theme, Theme::color());
        }
    }

    #[test]
    fn test_sort_arrow_rendering() {
        let mut app = App::new(
            SortOrder::Ping,
            GameMode::Standard,
            TimeFormat::Exact,
            vec![],
            Language::En,
            None,
        );
        app.queues = vec![api::RegionQueueData::new(
            "[DE]",
            "Frankfurt",
            "Standard",
            "10s",
            "6s",
        )];

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        let mut found_arrow = false;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Ping ▲") {
                found_arrow = true;
                break;
            }
        }
        assert!(found_arrow, "Active sort order Ping must display 'Ping ▲'");
    }

    #[test]
    fn test_border_margins_and_padding_prevent_border_collision() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Exact,
            vec!["eu-central-1".to_string()],
            Language::En,
            None,
        );
        app.queues = vec![
            api::RegionQueueData::new("[DE]", "Frankfurt", "Standard", "10s", "6s"),
            api::RegionQueueData::new("[IE]", "Dublin", "Standard", "12s", "8s"),
        ];
        app.pings.insert("eu-central-1".to_string(), 35);
        app.pings.insert("eu-west-1".to_string(), 45);

        // 1. Standard 80x24: Verify 2-space padding
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        // Line 1 is the header strip. Check that outer border is at x=0, and content starts at x=3.
        let l1: String = (0..buffer.area.width)
            .map(|x| buffer[(x, 1)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert_eq!(buffer[(0, 1)].symbol(), "│", "x=0 must be left border");
        assert_eq!(buffer[(1, 1)].symbol(), " ", "x=1 must be padding");
        assert_eq!(buffer[(2, 1)].symbol(), " ", "x=2 must be padding");
        let sort_byte = l1.find("Sort:").expect("Header should contain 'Sort:'");
        let sort_col = l1[..sort_byte].chars().count();
        assert_eq!(
            sort_col, 3,
            "Header content must start at column 3 with 2 padding spaces"
        );
        assert_eq!(buffer[(79, 1)].symbol(), "│", "x=79 must be right border");

        // Line 22 is footer strip. Verify padding on footer as well.
        assert_eq!(buffer[(0, 22)].symbol(), "│", "x=0 must be left border");
        assert_eq!(buffer[(1, 22)].symbol(), " ", "x=1 must be padding");
        assert_eq!(buffer[(2, 22)].symbol(), " ", "x=2 must be padding");

        // 2. Narrow 60x24: Verify 1-space padding
        let backend_narrow = TestBackend::new(60, 24);
        let mut term_narrow = Terminal::new(backend_narrow).unwrap();
        term_narrow.draw(|f| draw(f, &mut app)).unwrap();
        let buf_narrow = term_narrow.backend().buffer();

        let l1_narrow: String = (0..buf_narrow.area.width)
            .map(|x| buf_narrow[(x, 1)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert_eq!(buf_narrow[(0, 1)].symbol(), "│", "x=0 must be left border");
        assert_eq!(buf_narrow[(1, 1)].symbol(), " ", "x=1 must be padding");
        let sort_byte_narrow = l1_narrow
            .find("Sort:")
            .expect("Narrow header should contain 'Sort:'");
        let sort_col_narrow = l1_narrow[..sort_byte_narrow].chars().count();
        assert_eq!(
            sort_col_narrow, 2,
            "Narrow header content must start at column 2 with 1 padding space"
        );
    }

    #[test]
    fn test_table_and_summary_alignment_and_cohesion() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Exact,
            vec!["eu-central-1".to_string()],
            Language::En,
            None,
        );
        app.queues = vec![
            api::RegionQueueData::new("[DE]", "Frankfurt", "Standard", "10s", "6s"),
            api::RegionQueueData::new("[IE]", "Dublin", "Standard", "12s", "8s"),
        ];
        app.pings.insert("eu-central-1".to_string(), 35);
        app.pings.insert("eu-west-1".to_string(), 45);

        // 120x30 wide terminal: verify hero positioning, left alignment, and no massive gap
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        // Line 2 must be the Best Pick hero box top border
        let l2: String = (0..buffer.area.width)
            .map(|x| buffer[(x, 2)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(
            l2.contains("Best Pick"),
            "Line 2 must contain 'Best Pick' hero section"
        );
        assert!(
            l2.contains('┌'),
            "Line 2 must contain top-left border of Best Pick"
        );

        let summary_byte = l2.find('┌').unwrap();
        let summary_col = l2[..summary_byte].chars().count();

        // Line 6 must be the Table header
        let l6: String = (0..buffer.area.width)
            .map(|x| buffer[(x, 6)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(
            l6.contains("Region"),
            "Line 6 must contain 'Region' table header"
        );
        let table_byte = l6.find("Region").unwrap();
        let table_col = l6[..table_byte].chars().count();

        // Verify Best Pick, Table, and Header are horizontally aligned at column 3
        assert_eq!(
            summary_col, table_col,
            "Best Pick card left edge must align with Table header left edge"
        );
        assert_eq!(
            summary_col, 3,
            "Best Pick and Table must start at column 3 with 2 outer padding spaces"
        );

        // Verify internal padding inside Best Pick card: 'Killer:' starts at column 5 (3 + 1 border + 1 pad)
        let l3: String = (0..buffer.area.width)
            .map(|x| buffer[(x, 3)].symbol().chars().next().unwrap_or(' '))
            .collect();
        let killer_byte = l3.find("Killer:").expect("Killer row inside Best Pick");
        let killer_col = l3[..killer_byte].chars().count();
        assert_eq!(
            killer_col, 5,
            "Internal padding inside Best Pick card must place content at column 5"
        );

        // Check line 9 (data row with Frankfurt): gap elimination
        let l9: String = (0..buffer.area.width)
            .map(|x| buffer[(x, 9)].symbol().chars().next().unwrap_or(' '))
            .collect();
        let reg_x = l9.find("[DE] Frankfurt").expect("Data row with Frankfurt");
        let ping_x = l9.find("35 ms").expect("Data row with 35 ms");

        let gap = ping_x - reg_x;
        assert!(
            gap < 35,
            "Distance between Region and Ping must be balanced and cohesive (<35), got: {}",
            gap
        );
        assert!(
            gap > 10,
            "Distance between Region and Ping should have proper spacing (>10), got: {}",
            gap
        );
    }

    #[test]
    fn test_minimum_floor_48x12() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            TimeFormat::Exact,
            vec![],
            Language::En,
            None,
        );
        app.queues = vec![
            api::RegionQueueData::new("[DE]", "Frankfurt", "Standard", "10s", "6s"),
            api::RegionQueueData::new("[IE]", "Dublin", "Standard", "12s", "8s"),
        ];

        let backend = TestBackend::new(48, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        let mut found_header = false;
        let mut found_table_row = false;
        let mut found_footer = false;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            if line.contains("Sort:") {
                found_header = true;
            }
            if line.contains("Frankfurt") {
                found_table_row = true;
            }
            if line.contains("nav") && line.contains("quit") {
                found_footer = true;
            }
        }

        assert!(
            found_header,
            "Header must be rendered at 48x12 minimum floor"
        );
        assert!(
            found_table_row,
            "Table row must be rendered at 48x12 minimum floor"
        );
        assert!(
            found_footer,
            "Footer must be rendered at 48x12 minimum floor"
        );
    }
}
