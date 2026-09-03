use std::{io, time::Duration};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::sync::mpsc;
use std::thread;

pub enum AppEvent {
    Input(KeyCode),
    Tick,
    ApiUpdate(Result<(Vec<crate::api::RegionQueueData>, i64), String>),
    PingUpdate(std::collections::HashMap<String, u32>),
    ManualRefreshComplete {
        api_res: Result<(Vec<crate::api::RegionQueueData>, i64), String>,
        ping_res: std::collections::HashMap<String, u32>,
    },
}

pub fn run_app(mut app: crate::app::App) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(250);

    // Input thread
    let tx_input = tx.clone();
    thread::spawn(move || {
        let mut last_tick = std::time::Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).unwrap_or(false)
                && let Ok(Event::Key(key)) = event::read() {
                    if key.kind == crossterm::event::KeyEventKind::Press || key.kind == crossterm::event::KeyEventKind::Repeat {
                        tx_input.send(AppEvent::Input(key.code)).unwrap_or(());
                    }
                }

            if last_tick.elapsed() >= tick_rate {
                tx_input.send(AppEvent::Tick).unwrap_or(());
                last_tick = std::time::Instant::now();
            }
        }
    });

    // Fetch thread (every 60s)
    let tx_api = tx.clone();
    thread::spawn(move || {
        loop {
            let res = crate::api::fetch_queue_times();
            tx_api.send(AppEvent::ApiUpdate(res)).unwrap_or(());
            thread::sleep(Duration::from_secs(60));
        }
    });

    // Ping thread (every 60s)
    let tx_ping = tx.clone();
    thread::spawn(move || {
        loop {
            let pings = crate::ping::measure_all_regions_ping();
            tx_ping.send(AppEvent::PingUpdate(pings)).unwrap_or(());
            thread::sleep(Duration::from_secs(60));
        }
    });

    loop {
        terminal.draw(|f| crate::app::draw(f, &mut app))?;

        if let Ok(mut event) = rx.recv() {
            loop {
                match event {
                    AppEvent::Input(key) => match key {
                        KeyCode::Char(c) => {
                            if app.handle_key(c) == crate::app::AppAction::Refresh {
                                let tx_refresh = tx.clone();
                                thread::spawn(move || {
                                    let api_handle = thread::spawn(|| crate::api::fetch_queue_times());
                                    let ping_handle = thread::spawn(|| crate::ping::measure_all_regions_ping());
                                    let api_res = api_handle.join().unwrap_or_else(|_| Err("API fetch thread error".to_string()));
                                    let ping_res = ping_handle.join().unwrap_or_default();
                                    tx_refresh.send(AppEvent::ManualRefreshComplete { api_res, ping_res }).unwrap_or(());
                                });
                            }
                        }
                        KeyCode::Up => app.handle_up(),
                        KeyCode::Down => app.handle_down(),
                        KeyCode::Enter => app.handle_enter(),
                        KeyCode::Esc => app.handle_esc(),
                        _ => {}
                    },
                    AppEvent::Tick => {
                        app.on_tick();
                    }
                    AppEvent::ManualRefreshComplete { api_res, ping_res } => {
                        app.is_fetching = false;
                        app.status_msg = None;
                        app.pings = ping_res;
                        match api_res {
                            Ok((queues, last_updated)) => {
                                let is_same = app.api_last_updated == last_updated && !app.queues.is_empty();
                                app.queues = queues;
                                app.api_last_updated = last_updated;
                                app.error_msg = None;
                                let is_ru = std::env::var("LANG").unwrap_or_default().to_lowercase().starts_with("ru")
                                    || std::env::var("LC_ALL").unwrap_or_default().to_lowercase().starts_with("ru")
                                    || std::env::var("LC_MESSAGES").unwrap_or_default().to_lowercase().starts_with("ru");
                                let feedback = if is_same {
                                    if is_ru { "[✓ Актуально]" } else { "[✓ Up to date]" }
                                } else {
                                    if is_ru { "[✓ Обновлено]" } else { "[✓ Updated]" }
                                };
                                app.refresh_feedback = Some((feedback.to_string(), std::time::Instant::now()));
                            }
                            Err(e) => {
                                app.error_msg = Some(e);
                            }
                        }
                    }
                    AppEvent::ApiUpdate(res) => {
                        match res {
                            Ok((queues, last_updated)) => {
                                app.queues = queues;
                                app.api_last_updated = last_updated;
                                app.error_msg = None;
                            }
                            Err(e) => {
                                if app.queues.is_empty() {
                                    app.error_msg = Some(e);
                                }
                            }
                        }
                    }
                    AppEvent::PingUpdate(pings) => {
                        app.pings = pings;
                    }
                }

                if app.should_quit {
                    break;
                }

                match rx.try_recv() {
                    Ok(next) => event = next,
                    Err(_) => break,
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
