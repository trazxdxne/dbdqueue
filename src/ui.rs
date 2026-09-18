use crate::api;
use crate::app::{App, NoticeKind, SPINNER_FRAMES};
use crate::i18n::{TextKey, format_time_diff, tr, tr_mode, tr_sort, tr_time_format};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table},
};

pub const HEADER_HEIGHT: u16 = 3;
pub const FOOTER_HEIGHT: u16 = 1;
pub const LAYOUT_MARGIN: u16 = 1;
pub const EMPTY_TABLE_HEIGHT: u16 = 6;
pub const TABLE_CHROME_HEIGHT: u16 = 4;
pub const MODAL_WIDTH_PERCENT: u16 = 65;
pub const MODAL_HEIGHT_PERCENT: u16 = 80;
pub const TABLE_WIDTH: u16 = 24 + 12 + 14 + 14 + 3 + 2;
pub const SUMMARY_HEIGHT: u16 = 4;
pub const SUMMARY_LABEL_WIDTH: usize = 11;
pub const SUMMARY_TIME_WIDTH: usize = 8;
pub const SUMMARY_REGION_WIDTH: usize = 18;
pub const SUMMARY_PING_WIDTH: usize = 9;

pub fn color_for_time(time_str: &str) -> Color {
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

pub fn center_horizontally(area: Rect, width: u16) -> Rect {
    if area.width < width {
        area
    } else {
        Layout::horizontal([Constraint::Length(width)])
            .flex(Flex::Center)
            .split(area)[0]
    }
}

fn pad_right(s: &str, width: usize) -> String {
    let pad = width.saturating_sub(s.chars().count());
    format!("{}{}", s, " ".repeat(pad))
}

pub fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let active_sort_str = tr_sort(app.locale, app.sort);
    let mode_str = tr_mode(app.locale, app.mode);
    let lock_str = if app.locked.is_empty() {
        tr(app.locale, TextKey::LockNone)
    } else {
        tr(app.locale, TextKey::LockActive)
    };

    let mut spans = vec![
        Span::styled(
            tr(app.locale, TextKey::SortLabel),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            active_sort_str,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            tr(app.locale, TextKey::ModeLabel),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            mode_str,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
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
            spans.push(Span::styled(text, Style::default().fg(Color::LightYellow)));
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
            spans.push(Span::styled(text, Style::default().fg(Color::DarkGray)));
        }
    }

    spans.extend([
        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            tr(app.locale, TextKey::TimeLabel),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            tr_time_format(app.locale, app.time_format),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            tr(app.locale, TextKey::LockLabel),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            lock_str,
            if !app.locked.is_empty() {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
    ]);

    let header = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightRed))
            .title(Span::styled(
                tr(app.locale, TextKey::HeaderTitle),
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(header, area);
}

pub fn draw_table(f: &mut Frame, app: &mut App, area: Rect) {
    let rows_data = app.get_filtered_sorted_rows();
    let is_empty = rows_data.is_empty();

    let api_to_aws = api::get_api_to_aws();
    let (rows, col_constraints) = if is_empty {
        let (msg, color) = if app.is_fetching {
            (
                tr(app.locale, TextKey::FetchingQueues).to_string(),
                Color::LightRed,
            )
        } else if app
            .notice
            .as_ref()
            .is_some_and(|n| n.kind == NoticeKind::Error)
        {
            (
                tr(app.locale, TextKey::FailedQueues).to_string(),
                Color::Red,
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
                        Color::LightYellow,
                    )
                } else {
                    (
                        format!("  {}", tr(app.locale, TextKey::NoDataForMode)),
                        Color::DarkGray,
                    )
                }
            } else {
                (
                    format!("  {}", tr(app.locale, TextKey::NoDataForMode)),
                    Color::DarkGray,
                )
            }
        } else {
            (
                tr(app.locale, TextKey::NoDataForMode).to_string(),
                Color::DarkGray,
            )
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
        let r: Vec<Row> = rows_data
            .into_iter()
            .map(|item| {
                let aws_code = api_to_aws.get(item.name.as_str()).unwrap_or(&"");
                let is_whitelisted = app.locked.contains(*aws_code);
                let is_disabled = item.is_disabled();

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
                        Style::default()
                            .fg(Color::LightRed)
                            .add_modifier(Modifier::BOLD)
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
                        Cell::from(Span::styled(
                            item.survivor.clone(),
                            Style::default().fg(surv_color),
                        )),
                        Cell::from(Span::styled(
                            item.killer.clone(),
                            Style::default().fg(kill_color),
                        )),
                    ])
                }
            })
            .collect();
        (
            r,
            [
                Constraint::Length(24),
                Constraint::Length(12),
                Constraint::Length(14),
                Constraint::Length(14),
            ],
        )
    };

    let hdr_region = tr(app.locale, TextKey::ColRegion);
    let hdr_ping = tr(app.locale, TextKey::ColPing);
    let hdr_survivor = tr(app.locale, TextKey::ColSurvivor);
    let hdr_killer = tr(app.locale, TextKey::ColKiller);

    let mut table = Table::new(rows, col_constraints)
        .header(
            Row::new(vec![hdr_region, hdr_ping, hdr_survivor, hdr_killer])
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .column_spacing(1);

    if !is_empty {
        table = table.row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    }

    f.render_stateful_widget(table, area, &mut app.table_state);
}

pub fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let now = chrono::Utc::now().timestamp();
    let diff = now - app.api_last_updated;
    let time_str = if app.api_last_updated == 0 {
        tr(app.locale, TextKey::TimeFetching).to_string()
    } else {
        format_time_diff(app.locale, diff)
    };

    let status_span = if let Some(ref notice) = app.notice {
        let color = match notice.kind {
            NoticeKind::Error => Color::Red,
            NoticeKind::Info => Color::LightRed,
            NoticeKind::Success => Color::Green,
        };
        Span::styled(
            notice.message.clone(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )
    } else if app.is_fetching {
        let spinner = SPINNER_FRAMES[app.spinner_frame % SPINNER_FRAMES.len()];
        let msg = tr(app.locale, TextKey::StatusFetching);
        Span::styled(
            format!("{} {}", spinner, msg),
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        let prefix = tr(app.locale, TextKey::StatusApiUpdated);
        Span::styled(
            format!("{}{}", prefix, time_str),
            Style::default().fg(Color::DarkGray),
        )
    };

    let select_txt = tr(app.locale, TextKey::ActionSelect);
    let lock_txt = tr(app.locale, TextKey::ActionLock);
    let sort_txt = tr(app.locale, TextKey::ActionSort);
    let mode_txt = tr(app.locale, TextKey::ActionMode);
    let time_txt = tr(app.locale, TextKey::ActionTime);
    let refresh_txt = tr(app.locale, TextKey::ActionRefresh);
    let quit_txt = tr(app.locale, TextKey::ActionQuit);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" [↑↓] ", Style::default().fg(Color::LightRed)),
        Span::raw(select_txt),
        Span::styled(" [L] ", Style::default().fg(Color::LightRed)),
        Span::raw(lock_txt),
        Span::styled(" [S] ", Style::default().fg(Color::LightRed)),
        Span::raw(sort_txt),
        Span::styled(" [M] ", Style::default().fg(Color::LightRed)),
        Span::raw(mode_txt),
        Span::styled(" [T] ", Style::default().fg(Color::LightRed)),
        Span::raw(time_txt),
        Span::styled(" [R] ", Style::default().fg(Color::LightRed)),
        Span::raw(refresh_txt),
        Span::styled(" [Esc] ", Style::default().fg(Color::LightRed)),
        Span::raw(quit_txt),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        status_span,
    ]));
    f.render_widget(footer, area);
}

pub fn draw_lock_modal(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Clear, area);

    let modal_title = tr(app.locale, TextKey::ModalTitle);
    let select_txt = tr(app.locale, TextKey::ModalSelect);
    let toggle_txt = tr(app.locale, TextKey::ModalToggle);
    let apply_txt = tr(app.locale, TextKey::ModalApply);
    let cancel_txt = tr(app.locale, TextKey::ModalCancel);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightRed))
        .title(Span::styled(
            modal_title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

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
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
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

    // UX Fix 2: Modal action line rendered as a dedicated paragraph with red brackets and default fg text
    let modal_instructions = Line::from(vec![
        Span::styled(" [↑↓] ", Style::default().fg(Color::LightRed)),
        Span::styled(select_txt, Style::default().fg(Color::Reset)),
        Span::styled("  [Space] ", Style::default().fg(Color::LightRed)),
        Span::styled(toggle_txt, Style::default().fg(Color::Reset)),
        Span::styled("  [Enter] ", Style::default().fg(Color::LightRed)),
        Span::styled(apply_txt, Style::default().fg(Color::Reset)),
        Span::styled("  [Esc] ", Style::default().fg(Color::LightRed)),
        Span::styled(cancel_txt, Style::default().fg(Color::Reset)),
    ]);
    let footer_widget = Paragraph::new(modal_instructions);
    f.render_widget(footer_widget, modal_chunks[1]);
}

pub fn draw_summary(f: &mut Frame, app: &App, area: Rect) {
    if area.height < SUMMARY_HEIGHT {
        return;
    }

    let centered_area = center_horizontally(area, TABLE_WIDTH);

    let summary_area = Rect {
        x: centered_area.x,
        y: centered_area.y,
        width: centered_area.width,
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
            Style::default().fg(Color::DarkGray),
        );
        if let Some(best) = pick {
            let time_str = role.queue_time(best.row);
            let time_color = color_for_time(time_str);
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
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };
            let name_span = Span::styled(pad_right(&reg_str, SUMMARY_REGION_WIDTH), name_style);

            let ping = app.pings.get(*aws_code).copied();
            let (ping_text, ping_color) = if let Some(ms) = ping {
                (format!("{} ms", ms), crate::ping::color_for_ping(Some(ms)))
            } else {
                ("—".to_string(), Color::DarkGray)
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
                spans.push(Span::styled(sim_str, Style::default().fg(Color::DarkGray)));
            }

            Line::from(spans)
        } else {
            Line::from(vec![
                label_span,
                Span::styled("—", Style::default().fg(Color::DarkGray)),
            ])
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
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            format!(" {} ", tr(app.locale, TextKey::SummaryTitle)),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(vec![killer_line, survivor_line]).block(block);
    f.render_widget(paragraph, summary_area);
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let rows_data = app.get_filtered_sorted_rows();
    let is_empty = rows_data.is_empty();
    let desired_table_height = if is_empty {
        EMPTY_TABLE_HEIGHT
    } else {
        (rows_data.len() as u16) + TABLE_CHROME_HEIGHT
    };

    let margin = if f.area().height < 20 {
        0
    } else {
        LAYOUT_MARGIN
    };
    let non_table_fixed = HEADER_HEIGHT + SUMMARY_HEIGHT + FOOTER_HEIGHT + 2 * margin;

    let (show_summary, table_height) = if f.area().height >= non_table_fixed + 4 {
        let available_for_table = f.area().height.saturating_sub(non_table_fixed);
        (true, desired_table_height.min(available_for_table))
    } else {
        let available_without_summary = f
            .area()
            .height
            .saturating_sub(HEADER_HEIGHT + FOOTER_HEIGHT + 2 * margin);
        (false, desired_table_height.min(available_without_summary))
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(margin)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Length(table_height),
            if show_summary {
                Constraint::Min(SUMMARY_HEIGHT)
            } else {
                Constraint::Min(0)
            },
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(f.area());

    let table_area = center_horizontally(chunks[1], TABLE_WIDTH);

    draw_header(f, app, chunks[0]);
    draw_table(f, app, table_area);
    if show_summary {
        draw_summary(f, app, chunks[2]);
    }
    draw_footer(f, app, chunks[3]);

    if app.show_lock_modal {
        let modal_area = centered_rect(MODAL_WIDTH_PERCENT, MODAL_HEIGHT_PERCENT, f.area());
        draw_lock_modal(f, app, modal_area);
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

        // Search the buffer for the modal action line containing "[↑↓]" and "Select"
        let mut found_line = false;
        for y in 0..buffer.area.height {
            let line_chars: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();

            if line_chars.contains("[↑↓]") && line_chars.contains("Select") {
                found_line = true;
                // Find column index of '[' in "[↑↓]"
                let byte_bracket = line_chars.find('[').unwrap();
                let bracket_x = line_chars[..byte_bracket].chars().count() as u16;
                let bracket_cell = &buffer[(bracket_x, y)];
                assert_eq!(
                    bracket_cell.fg,
                    Color::LightRed,
                    "Bracket cell '[' should be LightRed"
                );

                // Find column index of 'S' in "Select"
                let byte_select = line_chars.find("Select").unwrap();
                let select_x = line_chars[..byte_select].chars().count() as u16;
                let text_cell = &buffer[(select_x, y)];
                assert_ne!(
                    text_cell.fg,
                    Color::LightRed,
                    "Label text 'Select' must not inherit LightRed border color"
                );
                assert_eq!(
                    text_cell.fg,
                    Color::Reset,
                    "Label text 'Select' should be default/Reset fg"
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
            .draw(|f| draw_summary(f, &app, Rect::new(0, 0, 80, 3)))
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
            .draw(|f| draw_summary(f, &app, Rect::new(0, 0, 80, 10)))
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
            .draw(|f| draw_summary(f, &app, Rect::new(0, 0, 80, 10)))
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

        let l0: String = (0..80)
            .map(|x| buffer[(x, 2)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(
            l0.contains("Time: ") && l0.contains("Exact"),
            "Header must display 'Time: Exact', got: '{}'",
            l0
        );

        // Toggle to rounded
        app.toggle_time_format();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer2 = terminal.backend().buffer();
        let l0_rounded: String = (0..80)
            .map(|x| buffer2[(x, 2)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(
            l0_rounded.contains("Time: ") && l0_rounded.contains("~Rounded"),
            "Header must display 'Time: ~Rounded', got: '{}'",
            l0_rounded
        );
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
        let header_line: String = (0..100)
            .map(|x| buffer[(x, 2)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(header_line.contains("Mode: Standard"));
        assert!(
            !header_line.contains("2v8 event"),
            "Event badge must not appear in Standard mode"
        );

        // 2. Event mode: event badge appears right after Mode: Event
        app.mode = GameMode::Event;
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer2 = terminal.backend().buffer();
        let header_line_event: String = (0..100)
            .map(|x| buffer2[(x, 2)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(header_line_event.contains("Mode: Event [2v8 event - 11d left]"));
        assert!(header_line_event.contains("│  Time: ~Rounded"));
    }
}
