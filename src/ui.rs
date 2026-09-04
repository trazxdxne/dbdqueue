use crate::api;
use crate::app::{App, NoticeKind, SPINNER_FRAMES};
use crate::i18n::{TextKey, format_time_diff, tr, tr_mode, tr_sort};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table},
};

pub const HEADER_HEIGHT: u16 = 3;
pub const FOOTER_HEIGHT: u16 = 1;
pub const LAYOUT_MARGIN: u16 = 1;
pub const EMPTY_TABLE_HEIGHT: u16 = 6;
pub const TABLE_CHROME_HEIGHT: u16 = 4;
pub const MODAL_WIDTH_PERCENT: u16 = 65;
pub const MODAL_HEIGHT_PERCENT: u16 = 80;
pub const TABLE_WIDTH: u16 = 24 + 12 + 14 + 14 + 3 + 2;

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

pub fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let active_sort_str = tr_sort(app.locale, app.sort);
    let mode_str = tr_mode(app.locale, app.mode);
    let lock_str = if app.locked.is_empty() {
        tr(app.locale, TextKey::LockNone)
    } else {
        tr(app.locale, TextKey::LockActive)
    };

    let header_line = Line::from(vec![
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

    let header = Paragraph::new(header_line).block(
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
            (tr(app.locale, TextKey::FetchingQueues), Color::LightRed)
        } else if app
            .notice
            .as_ref()
            .is_some_and(|n| n.kind == NoticeKind::Error)
        {
            (tr(app.locale, TextKey::FailedQueues), Color::Red)
        } else {
            (tr(app.locale, TextKey::NoDataForMode), Color::DarkGray)
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
    f.render_widget(list, modal_chunks[0]);

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
    if area.height < 5 {
        return;
    }

    let summary_area = if area.width < TABLE_WIDTH {
        area
    } else {
        Layout::horizontal([Constraint::Length(TABLE_WIDTH)])
            .flex(Flex::Center)
            .split(area)[0]
    };

    let summary = app.summary();
    let api_to_aws = api::get_api_to_aws();

    let format_label = |key: TextKey| -> String {
        let label_str = tr(app.locale, key);
        let pad = 11usize.saturating_sub(label_str.chars().count());
        format!("{}{}", label_str, " ".repeat(pad))
    };

    let format_pick_line =
        |key: TextKey, pick: Option<&crate::app::BestPick<'_>>, is_killer: bool| -> Line<'_> {
            let label_span = Span::styled(format_label(key), Style::default().fg(Color::DarkGray));
            if let Some(best) = pick {
                let time_str = if is_killer {
                    &best.row.killer
                } else {
                    &best.row.survivor
                };
                let time_color = color_for_time(time_str);
                let time_span = Span::styled(time_str.clone(), Style::default().fg(time_color));

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
                let name_span = Span::styled(reg_str, name_style);

                let ping = app.pings.get(*aws_code).copied();
                let (ping_text, ping_color) = if let Some(ms) = ping {
                    (
                        format!("({} ms)", ms),
                        crate::ping::color_for_ping(Some(ms)),
                    )
                } else {
                    ("(—)".to_string(), Color::DarkGray)
                };
                let ping_span = Span::styled(ping_text, Style::default().fg(ping_color));

                let mut spans = vec![
                    label_span,
                    time_span,
                    Span::raw("  "),
                    name_span,
                    Span::raw("  "),
                    ping_span,
                ];

                if best.similar > 0 {
                    spans.push(Span::raw("  "));
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

    let killer_line = format_pick_line(TextKey::SummaryKiller, summary.killer.as_ref(), true);
    let survivor_line =
        format_pick_line(TextKey::SummarySurvivor, summary.survivor.as_ref(), false);

    let ping_line = {
        let label_span = Span::styled(
            format_label(TextKey::SummaryPing),
            Style::default().fg(Color::DarkGray),
        );
        if let Some(row) = summary.lowest_ping {
            let aws_code = api_to_aws.get(row.name.as_str()).unwrap_or(&"");
            let ping = app.pings.get(*aws_code).copied();
            let (ping_text, ping_color) = if let Some(ms) = ping {
                (format!("{} ms", ms), crate::ping::color_for_ping(Some(ms)))
            } else {
                ("—".to_string(), Color::DarkGray)
            };
            let ping_span = Span::styled(ping_text, Style::default().fg(ping_color));

            let reg_str = if row.flag.is_empty() {
                row.name.clone()
            } else {
                format!("{} {}", row.flag, row.name)
            };
            let is_locked = app.locked.contains(*aws_code);
            let name_style = if is_locked {
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };
            let name_span = Span::styled(reg_str, name_style);

            Line::from(vec![label_span, ping_span, Span::raw("  "), name_span])
        } else {
            Line::from(vec![
                label_span,
                Span::styled("—", Style::default().fg(Color::DarkGray)),
            ])
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            format!(" {} ", tr(app.locale, TextKey::SummaryTitle)),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(vec![killer_line, survivor_line, ping_line]).block(block);
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

    // Prevent table from pushing footer off small terminal screens
    let available_height = f
        .area()
        .height
        .saturating_sub(HEADER_HEIGHT + FOOTER_HEIGHT + 2 * LAYOUT_MARGIN);
    let table_height = desired_table_height.min(available_height);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(LAYOUT_MARGIN)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Length(table_height),
            Constraint::Min(0),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(f.area());

    let table_area = if chunks[1].width < TABLE_WIDTH {
        chunks[1]
    } else {
        Layout::horizontal([Constraint::Length(TABLE_WIDTH)])
            .flex(Flex::Center)
            .split(chunks[1])[0]
    };

    draw_header(f, app, chunks[0]);
    draw_table(f, app, table_area);
    draw_summary(f, app, chunks[2]);
    draw_footer(f, app, chunks[3]);

    if app.show_lock_modal {
        let modal_area = centered_rect(MODAL_WIDTH_PERCENT, MODAL_HEIGHT_PERCENT, f.area());
        draw_lock_modal(f, app, modal_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GameMode, Language, SortOrder};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_modal_label_color_via_test_backend() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            vec![],
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
            vec![],
            vec!["eu-central-1".to_string()],
            Language::En,
            None,
        );
        app.queues = vec![
            api::RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "6s".to_string(),
            },
            api::RegionQueueData {
                flag: "[IE]".to_string(),
                name: "Dublin".to_string(),
                mode: "Standard".to_string(),
                survivor: "12s".to_string(),
                killer: "8s".to_string(),
            },
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
            if line.contains("Best pick now") {
                found_summary = true;
                break;
            }
        }
        assert!(found_summary, "Summary panel should be rendered in buffer");
    }

    #[test]
    fn test_draw_summary_russian_and_skip_height() {
        let mut app = App::new(
            SortOrder::Default,
            GameMode::Standard,
            vec![],
            vec!["eu-central-1".to_string()],
            Language::Ru,
            None,
        );
        app.queues = vec![
            api::RegionQueueData {
                flag: "[DE]".to_string(),
                name: "Frankfurt".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "6s".to_string(),
            },
            api::RegionQueueData {
                flag: "[IE]".to_string(),
                name: "Dublin".to_string(),
                mode: "Standard".to_string(),
                survivor: "10s".to_string(),
                killer: "6s".to_string(),
            },
        ];
        app.pings.insert("eu-central-1".to_string(), 35);
        app.pings.insert("eu-west-1".to_string(), 45);

        // Test skip rendering when height < 5
        let backend_small = TestBackend::new(80, 4);
        let mut term_small = Terminal::new(backend_small).unwrap();
        term_small
            .draw(|f| draw_summary(f, &app, Rect::new(0, 0, 80, 4)))
            .unwrap();
        let buf_small = term_small.backend().buffer();
        for y in 0..4 {
            let line: String = (0..80)
                .map(|x| buf_small[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            assert!(!line.contains("Лучший выбор сейчас"));
        }

        // Test Russian rendering when height >= 5
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
            if line.contains("Лучший выбор сейчас") {
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
    }
}
