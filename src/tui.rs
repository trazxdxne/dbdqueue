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

    loop {
        terminal.draw(|f| crate::app::draw(f, &mut app))?;

        if let Ok(event) = rx.recv() {
            match event {
                AppEvent::Input(key) => match key {
                    KeyCode::Char(c) => app.handle_key(c),
                    KeyCode::Up => app.previous(),
                    KeyCode::Down => app.next(),
                    KeyCode::Esc => app.should_quit = true,
                    _ => {}
                },
                AppEvent::Tick => {}
                AppEvent::ApiUpdate(res) => {
                    app.is_fetching = false;
                    match res {
                        Ok((queues, last_updated)) => {
                            app.queues = queues;
                            app.api_last_updated = last_updated;
                            app.error_msg = None;
                        }
                        Err(e) => {
                            app.error_msg = Some(e);
                        }
                    }
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
