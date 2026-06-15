use std::io::{self, Write};
use regex::Regex;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crate::api::RegionQueueData;

const C_RESET: &str = "\x1b[0m";
const C_BOLD: &str = "\x1b[1m";
const C_GREEN: &str = "\x1b[92m";
const C_YELLOW: &str = "\x1b[93m";
const C_RED: &str = "\x1b[91m";
const C_CYAN: &str = "\x1b[96m";
const C_GRAY: &str = "\x1b[90m";

fn is_russian() -> bool {
    let lang = std::env::var("LANG").unwrap_or_default().to_lowercase();
    let lc_all = std::env::var("LC_ALL").unwrap_or_default().to_lowercase();
    let lc_msg = std::env::var("LC_MESSAGES").unwrap_or_default().to_lowercase();
    
    lang.starts_with("ru") || lc_all.starts_with("ru") || lc_msg.starts_with("ru")
}

pub fn get_terminal_width(s: &str) -> usize {
    let ansi_escape = Regex::new(r"\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])").unwrap();
    let clean = ansi_escape.replace_all(s, "");
    
    let mut width = 0;
    for c in clean.chars() {
        let code = c as u32;
        if (0x1F1E6..=0x1F1FF).contains(&code) {
            width += 1;
        } else if (0x1F300..=0x1F9FF).contains(&code) || (0x2600..=0x27BF).contains(&code) {
            width += 2;
        } else {
            width += 1;
        }
    }
    width
}

pub fn pad_right(s: &str, target_width: usize) -> String {
    let current_w = get_terminal_width(s);
    if current_w >= target_width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(target_width - current_w))
    }
}

pub fn colorize_time(time_str: &str) -> String {
    if time_str == "—" || time_str.is_empty() {
        return format!("{}—{}", C_GRAY, C_RESET);
    }
    let sec = crate::api::parse_time_to_seconds(time_str);
    if sec < 60 {
        format!("{}{}{}", C_GREEN, time_str, C_RESET)
    } else if sec < 180 {
        format!("{}{}{}", C_YELLOW, time_str, C_RESET)
    } else {
        format!("{}{}{}", C_RED, time_str, C_RESET)
    }
}

pub fn draw_table(rows: &[RegionQueueData], priority_list: &[String], api_last_updated: i64) {
    let is_ru = is_russian();
    let col_width_region = 22;
    let col_width_mode = 10;
    let col_width_surv = 12;
    let col_width_kill = 12;
    
    let border_top = format!(
        "┌{}┬{}┬{}┬{}┐",
        "─".repeat(col_width_region + 2),
        "─".repeat(col_width_mode + 2),
        "─".repeat(col_width_surv + 2),
        "─".repeat(col_width_kill + 2)
    );
    let border_mid = format!(
        "├{}┼{}┼{}┼{}┤",
        "─".repeat(col_width_region + 2),
        "─".repeat(col_width_mode + 2),
        "─".repeat(col_width_surv + 2),
        "─".repeat(col_width_kill + 2)
    );
    let border_double = format!(
        "╞{}╪{}╪{}╪{}╡",
        "═".repeat(col_width_region + 2),
        "═".repeat(col_width_mode + 2),
        "═".repeat(col_width_surv + 2),
        "═".repeat(col_width_kill + 2)
    );
    let border_bot = format!(
        "└{}┴{}┴{}┴{}┘",
        "─".repeat(col_width_region + 2),
        "─".repeat(col_width_mode + 2),
        "─".repeat(col_width_surv + 2),
        "─".repeat(col_width_kill + 2)
    );
    
    println!("{}", border_top);
    
    let txt_region = if is_ru { "Регион" } else { "Region" };
    let txt_mode = if is_ru { "Режим" } else { "Mode" };
    let txt_surv = if is_ru { "Выживший" } else { "Survivor" };
    let txt_kill = if is_ru { "Маньяк" } else { "Killer" };
    
    let r_hdr = pad_right(&format!("{}{}{}", C_BOLD, txt_region, C_RESET), col_width_region);
    let m_hdr = pad_right(&format!("{}{}{}", C_BOLD, txt_mode, C_RESET), col_width_mode);
    let s_hdr = pad_right(&format!("{}{}{}", C_BOLD, txt_surv, C_RESET), col_width_surv);
    let k_hdr = pad_right(&format!("{}{}{}", C_BOLD, txt_kill, C_RESET), col_width_kill);
    println!("│ {} │ {} │ {} │ {} │", r_hdr, m_hdr, s_hdr, k_hdr);
    println!("{}", border_double);
    
    for (i, row) in rows.iter().enumerate() {
        let flag_spaced = if row.flag.is_empty() {
            String::new()
        } else {
            format!("{} ", row.flag)
        };
        let mut reg_text = format!("{}{}", flag_spaced, row.name);
        if priority_list.contains(&row.name) {
            reg_text = format!("{}{}{}", C_CYAN, reg_text, C_RESET);
        }
        
        let r_cell = pad_right(&reg_text, col_width_region);
        let mode_translated = if is_ru {
            match row.mode.as_str() {
                "Standard" => "Обычный",
                "Event" => "Ивент",
                other => other,
            }
        } else {
            row.mode.as_str()
        };
        let m_cell = pad_right(mode_translated, col_width_mode);
        
        let s_val = colorize_time(&row.survivor);
        let k_val = colorize_time(&row.killer);
        
        let s_pad = col_width_surv.saturating_sub(get_terminal_width(&row.survivor));
        let k_pad = col_width_kill.saturating_sub(get_terminal_width(&row.killer));
        
        let s_colored = format!("{}{}", s_val, " ".repeat(s_pad));
        let k_colored = format!("{}{}", k_val, " ".repeat(k_pad));
        
        println!("│ {} │ {} │ {} │ {} │", r_cell, m_cell, s_colored, k_colored);
        if i < rows.len() - 1 {
            println!("{}", border_mid);
        }
    }
    println!("{}", border_bot);
    
    let now = chrono::Utc::now().timestamp();
    let diff = now - api_last_updated;
    let txt_just_now = if is_ru { "только что" } else { "just now" };
    let txt_updated = if is_ru { "Обновлено: " } else { "Updated: " };
    
    let time_str = if diff < 60 {
        txt_just_now.to_string()
    } else if diff < 3600 {
        let mins = diff / 60;
        if is_ru {
            format!("{} мин. назад", mins)
        } else {
            format!("{} mins ago", mins)
        }
    } else {
        let hours = diff / 3600;
        let mins = (diff % 3600) / 60;
        if is_ru {
            format!("{} ч. {} мин. назад", hours, mins)
        } else {
            format!("{} hrs {} mins ago", hours, mins)
        }
    };
    
    println!("{}{}{}{}", C_GRAY, txt_updated, time_str, C_RESET);
}



fn run_interactive_menu(
    title: &str,
    options: &[(String, String)],
    initial_selected: &[String],
    instructions: &str,
) -> Option<Vec<String>> {
    let mut selected: Vec<bool> = options.iter().map(|(_, val)| initial_selected.contains(val)).collect();
    let mut cursor_pos = 0;
    
    let mut stdout = io::stdout();
    enable_raw_mode().ok()?;
    
    // Hide cursor and clear screen
    crossterm::execute!(stdout, crossterm::cursor::Hide, crossterm::terminal::Clear(crossterm::terminal::ClearType::All)).ok();
    
    loop {
        crossterm::execute!(stdout, crossterm::cursor::MoveTo(0,0), crossterm::terminal::Clear(crossterm::terminal::ClearType::All)).ok();
        write!(stdout, "{}{}{}{}\r\n", C_BOLD, C_CYAN, title, C_RESET).ok();
        write!(stdout, "{}\r\n\r\n", instructions).ok();
        
        for (i, (display, code)) in options.iter().enumerate() {
            let checked = if selected[i] { "[*]" } else { "[ ]" };
            let color = if selected[i] { C_GREEN } else { C_GRAY };
            
            if i == cursor_pos {
                write!(stdout, " \x1b[7m {} {} ({}) \x1b[27m\r\n", checked, display, code).ok();
            } else {
                write!(stdout, " {} {} {} ({}){}\r\n", color, checked, display, code, C_RESET).ok();
            }
        }
        stdout.flush().ok();
        
        if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
            match code {
                KeyCode::Up => {
                    cursor_pos = (cursor_pos + options.len() - 1) % options.len();
                }
                KeyCode::Down => {
                    cursor_pos = (cursor_pos + 1) % options.len();
                }
                KeyCode::Char(' ') => {
                    selected[cursor_pos] = !selected[cursor_pos];
                }
                KeyCode::Enter => {
                    break;
                }
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    let cancel_msg = if is_russian() { "\r\n\x1b[93mОтменено.\x1b[0m\r\n" } else { "\r\n\x1b[93mCancelled.\x1b[0m\r\n" };
                    disable_raw_mode().ok();
                    crossterm::execute!(stdout, crossterm::cursor::Show).ok();
                    write!(stdout, "{}", cancel_msg).ok();
                    stdout.flush().ok();
                    return None;
                }
                _ => {}
            }
        }
    }
    
    disable_raw_mode().ok();
    crossterm::execute!(stdout, crossterm::cursor::Show).ok();
    stdout.flush().ok();
    
    let result = options.iter().enumerate()
        .filter(|(i, _)| selected[*i])
        .map(|(_, (_, val))| val.clone())
        .collect();
    Some(result)
}

pub fn interactive_lock_menu(current_locked: &[String]) -> Option<Vec<String>> {
    let api_to_aws = crate::api::get_api_to_aws();
    let mut regions: Vec<String> = api_to_aws.keys().map(|s| s.to_string()).collect();
    regions.sort();
    
    let options: Vec<(String, String)> = regions.iter()
        .map(|r| (r.clone(), api_to_aws.get(r.as_str()).unwrap().to_string()))
        .collect();
        
    let is_ru = is_russian();
    let title = if is_ru { "=== DBD Блокировка Регионов (Интерактивный Режим) ===" } else { "=== DBD Region Locker (Interactive Mode) ===" };
    let instructions = if is_ru { "Навигация: СТРЕЛКИ | Переключить: ПРОБЕЛ | Сохранить: ENTER | Выход: Ctrl+C" } else { "Navigate: ARROWS | Toggle: SPACE | Lock & Save: ENTER | Quit: Ctrl+C" };
        
    run_interactive_menu(
        title,
        &options,
        current_locked,
        instructions,
    )
}

pub fn interactive_priority_menu(current_priority: &[String]) -> Option<Vec<String>> {
    let api_to_aws = crate::api::get_api_to_aws();
    let mut regions: Vec<String> = api_to_aws.keys().map(|s| s.to_string()).collect();
    regions.sort();
    
    let options: Vec<(String, String)> = regions.iter()
        .map(|r| (r.clone(), r.clone()))
        .collect();
        
    let is_ru = is_russian();
    let title = if is_ru { "=== DBD Приоритетные Регионы (Интерактивный Режим) ===" } else { "=== DBD Priority Regions (Interactive Mode) ===" };
    let instructions = if is_ru { "Навигация: СТРЕЛКИ | Переключить: ПРОБЕЛ | Сохранить: ENTER | Выход: Ctrl+C" } else { "Navigate: ARROWS | Toggle: SPACE | Save: ENTER | Quit: Ctrl+C" };
        
    run_interactive_menu(
        title,
        &options,
        current_priority,
        instructions,
    )
}
