use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Once;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

static PANIC_HOOK_ONCE: Once = Once::new();

/// Unconditionally resets raw mode, leaves alternate screen, disables mouse capture,
/// and restores cursor visibility safely without panicking.
pub fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        cursor::Show
    );
    let _ = io::stdout().flush();
}

/// Installs a panic hook that resets the terminal state before delegating
/// to the previous panic hook.
pub fn install_panic_hook() {
    PANIC_HOOK_ONCE.call_once(|| {
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            restore_terminal();
            previous_hook(panic_info);
        }));
    });
}

/// RAII guard ensuring the terminal is restored to normal cooked mode on drop.
pub struct TerminalGuard;

impl TerminalGuard {
    /// Enters raw mode, switches to alternate screen, enables mouse capture,
    /// and hides the cursor.
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(err) = execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            cursor::Hide
        ) {
            let _ = disable_raw_mode();
            return Err(err);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

pub enum AppEvent {
    Input(KeyEvent),
    Tick,
    #[allow(dead_code)]
    Resize(u16, u16),
    ApiUpdate(Result<(Vec<crate::api::RegionQueueData>, i64), String>),
    PingUpdate(std::collections::HashMap<String, u32>),
    MiscUpdate(Result<crate::api::MiscResponse, String>),
    ManualRefreshComplete {
        api_res: Result<(Vec<crate::api::RegionQueueData>, i64), String>,
        ping_res: std::collections::HashMap<String, u32>,
        misc_res: Result<crate::api::MiscResponse, String>,
    },
    HostsUpdateComplete {
        result: crate::hosts::UpdateHostsResult,
        locked: Vec<String>,
    },
}

pub fn run_app(
    mut app: crate::app::App,
    config_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    install_panic_hook();
    let _guard = TerminalGuard::enter()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let cache_path = crate::cache::get_cache_path(&config_path);

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

            if event::poll(timeout).unwrap_or(false) {
                match event::read() {
                    Ok(Event::Key(key)) => {
                        if key.kind == crossterm::event::KeyEventKind::Press
                            || key.kind == crossterm::event::KeyEventKind::Repeat
                        {
                            tx_input.send(AppEvent::Input(key)).unwrap_or(());
                        }
                    }
                    Ok(Event::Resize(w, h)) => {
                        tx_input.send(AppEvent::Resize(w, h)).unwrap_or(());
                    }
                    _ => {}
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

    // Misc/event thread (every 60s)
    let tx_misc = tx.clone();
    thread::spawn(move || {
        loop {
            let res = crate::api::fetch_misc_data();
            tx_misc.send(AppEvent::MiscUpdate(res)).unwrap_or(());
            thread::sleep(Duration::from_secs(60));
        }
    });

    let dispatch_action = |action: crate::app::AppAction,
                           app: &mut crate::app::App,
                           tx: &mpsc::Sender<AppEvent>| {
        match action {
            crate::app::AppAction::None => {}
            crate::app::AppAction::Refresh => {
                let tx_refresh = tx.clone();
                thread::spawn(move || {
                    let api_handle = thread::spawn(crate::api::fetch_queue_times);
                    let ping_handle = thread::spawn(crate::ping::measure_all_regions_ping);
                    let misc_handle = thread::spawn(crate::api::fetch_misc_data);
                    let api_res = api_handle
                        .join()
                        .unwrap_or_else(|_| Err("API fetch thread error".to_string()));
                    let ping_res = ping_handle.join().unwrap_or_default();
                    let misc_res = misc_handle
                        .join()
                        .unwrap_or_else(|_| Err("Misc fetch thread error".to_string()));
                    tx_refresh
                        .send(AppEvent::ManualRefreshComplete {
                            api_res,
                            ping_res,
                            misc_res,
                        })
                        .unwrap_or(());
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
                    let lock_target = if regions.is_empty() {
                        None
                    } else {
                        Some(regions.as_slice())
                    };
                    let res = crate::hosts::update_hosts(lock_target, false);
                    tx_hosts
                        .send(AppEvent::HostsUpdateComplete {
                            result: res,
                            locked: regions,
                        })
                        .unwrap_or(());
                });
            }
        }
    };

    loop {
        terminal.draw(|f| crate::ui::draw(f, &mut app))?;

        if let Ok(mut event) = rx.recv() {
            loop {
                match event {
                    AppEvent::Input(key) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('C'))
                        {
                            app.should_quit = true;
                            break;
                        }

                        match key.code {
                            KeyCode::Char(c) => {
                                let action = app.handle_key(c);
                                dispatch_action(action, &mut app, &tx);
                            }
                            KeyCode::Up => app.handle_up(),
                            KeyCode::Down => app.handle_down(),
                            KeyCode::Home => app.handle_home(),
                            KeyCode::End => app.handle_end(),
                            KeyCode::PageUp => app.handle_page_up(),
                            KeyCode::PageDown => app.handle_page_down(),
                            KeyCode::Enter => {
                                let action = app.handle_enter();
                                dispatch_action(action, &mut app, &tx);
                            }
                            KeyCode::Esc => app.handle_esc(),
                            _ => {}
                        }
                    }
                    AppEvent::Tick => {
                        app.on_tick(std::time::Instant::now());
                    }
                    AppEvent::Resize(_, _) => {
                        // Terminal resize triggers redraw at the top of loop
                    }
                    AppEvent::ManualRefreshComplete {
                        api_res,
                        ping_res,
                        misc_res,
                    } => {
                        app.handle_manual_refresh_complete(
                            api_res,
                            ping_res,
                            misc_res,
                            std::time::Instant::now(),
                        );
                        let _ = crate::cache::save_ping_cache(&cache_path, &app.pings);
                    }
                    AppEvent::HostsUpdateComplete { result, locked } => {
                        let action =
                            app.handle_hosts_result(result, locked, std::time::Instant::now());
                        dispatch_action(action, &mut app, &tx);
                    }
                    AppEvent::ApiUpdate(res) => {
                        app.handle_api_update(res);
                    }
                    AppEvent::PingUpdate(pings) => {
                        app.handle_ping_update(pings);
                        let _ = crate::cache::save_ping_cache(&cache_path, &app.pings);
                    }
                    AppEvent::MiscUpdate(res) => {
                        app.handle_misc_update(res);
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

    drop(_guard);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restore_terminal_does_not_panic() {
        restore_terminal();
    }

    #[test]
    fn test_terminal_guard_drop_idempotent() {
        let guard = TerminalGuard;
        drop(guard);
        restore_terminal();
    }

    #[test]
    fn test_install_panic_hook_idempotent() {
        install_panic_hook();
        install_panic_hook();
    }
}
