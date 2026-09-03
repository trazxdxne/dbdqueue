use std::path::PathBuf;
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
    HostsUpdateComplete {
        result: crate::hosts::UpdateHostsResult,
        locked: Vec<String>,
    },
}

pub fn run_app(mut app: crate::app::App, config_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
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
                && let Ok(Event::Key(key)) = event::read()
                && (key.kind == crossterm::event::KeyEventKind::Press || key.kind == crossterm::event::KeyEventKind::Repeat)
            {
                tx_input.send(AppEvent::Input(key.code)).unwrap_or(());
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

    let dispatch_action = |action: crate::app::AppAction, app: &mut crate::app::App, tx: &mpsc::Sender<AppEvent>| {
        match action {
            crate::app::AppAction::None => {}
            crate::app::AppAction::Refresh => {
                let tx_refresh = tx.clone();
                thread::spawn(move || {
                    let api_handle = thread::spawn(crate::api::fetch_queue_times);
                    let ping_handle = thread::spawn(crate::ping::measure_all_regions_ping);
                    let api_res = api_handle.join().unwrap_or_else(|_| Err("API fetch thread error".to_string()));
                    let ping_res = ping_handle.join().unwrap_or_default();
                    tx_refresh.send(AppEvent::ManualRefreshComplete { api_res, ping_res }).unwrap_or(());
                });
            }
            crate::app::AppAction::SaveConfig(cfg) => {
                if let Err(e) = crate::config::save_config(&config_path, &cfg) {
                    let prefix = crate::i18n::tr(app.locale, crate::i18n::TextKey::ErrorConfigSave);
                    app.notice = Some(crate::app::Notice::error(format!("{}: {}", prefix, e)));
                }
            }
            crate::app::AppAction::ApplyLocks(regions) => {
                let tx_hosts = tx.clone();
                thread::spawn(move || {
                    let lock_target = if regions.is_empty() { None } else { Some(regions.as_slice()) };
                    let res = crate::hosts::update_hosts(lock_target, false);
                    tx_hosts.send(AppEvent::HostsUpdateComplete { result: res, locked: regions }).unwrap_or(());
                });
            }
        }
    };

    loop {
        terminal.draw(|f| crate::ui::draw(f, &mut app))?;

        if let Ok(mut event) = rx.recv() {
            loop {
                match event {
                    AppEvent::Input(key) => match key {
                        KeyCode::Char(c) => {
                            let action = app.handle_key(c);
                            dispatch_action(action, &mut app, &tx);
                        }
                        KeyCode::Up => app.handle_up(),
                        KeyCode::Down => app.handle_down(),
                        KeyCode::Enter => {
                            let action = app.handle_enter();
                            dispatch_action(action, &mut app, &tx);
                        }
                        KeyCode::Esc => app.handle_esc(),
                        _ => {}
                    },
                    AppEvent::Tick => {
                        app.on_tick(std::time::Instant::now());
                    }
                    AppEvent::ManualRefreshComplete { api_res, ping_res } => {
                        app.handle_manual_refresh_complete(api_res, ping_res, std::time::Instant::now());
                    }
                    AppEvent::HostsUpdateComplete { result, locked } => {
                        let action = app.handle_hosts_result(result, locked, std::time::Instant::now());
                        dispatch_action(action, &mut app, &tx);
                    }
                    AppEvent::ApiUpdate(res) => {
                        app.handle_api_update(res);
                    }
                    AppEvent::PingUpdate(pings) => {
                        app.handle_ping_update(pings);
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
