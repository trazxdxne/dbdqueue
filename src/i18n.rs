use crate::config::{GameMode, Language, SortOrder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Ru,
}

pub fn resolve_locale_from_sources(
    config_lang: Language,
    sys_locale: Option<&str>,
    lc_all: Option<&str>,
    lc_messages: Option<&str>,
    lang: Option<&str>,
) -> Locale {
    match config_lang {
        Language::En => Locale::En,
        Language::Ru => Locale::Ru,
        Language::Auto => {
            // 1. Try sys_locale
            if let Some(sys) = sys_locale {
                let trimmed = sys.trim();
                if !trimmed.is_empty() {
                    return if is_russian_str(trimmed) {
                        Locale::Ru
                    } else {
                        Locale::En
                    };
                }
            }
            // 2. Fall back to LC_ALL > LC_MESSAGES > LANG (first non-empty wins)
            let env_val = lc_all
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .or_else(|| lc_messages.map(str::trim).filter(|s| !s.is_empty()))
                .or_else(|| lang.map(str::trim).filter(|s| !s.is_empty()));

            if let Some(val) = env_val
                && is_russian_str(val)
            {
                return Locale::Ru;
            }

            Locale::En
        }
    }
}

fn is_russian_str(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.starts_with("ru") || lower.contains("ru_") || lower.contains("ru-")
}

pub fn resolve_locale(config_lang: Language) -> Locale {
    let sys = sys_locale::get_locale();
    let lc_all = std::env::var("LC_ALL").ok();
    let lc_messages = std::env::var("LC_MESSAGES").ok();
    let lang = std::env::var("LANG").ok();
    resolve_locale_from_sources(
        config_lang,
        sys.as_deref(),
        lc_all.as_deref(),
        lc_messages.as_deref(),
        lang.as_deref(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKey {
    HeaderTitle,
    SortLabel,
    ModeLabel,
    LockLabel,
    LockNone,
    LockActive,
    ColRegion,
    ColPing,
    ColSurvivor,
    ColKiller,
    FetchingQueues,
    FailedQueues,
    NoDataForMode,
    ActionSelect,
    ActionLock,
    ActionSort,
    ActionMode,
    ActionRefresh,
    ActionQuit,
    TimeFetching,
    TimeJustNow,
    StatusFetching,
    StatusApiUpdated,
    FeedbackUpToDate,
    FeedbackUpdated,
    ErrorPrefix,
    ErrorConfigSave,
    ModalTitle,
    ModalSelect,
    ModalToggle,
    ModalApply,
    ModalCancel,
    HostsUpdated,
    HostsAlreadyUpToDate,
    HostsElevationFailed,
}

pub fn tr(locale: Locale, key: TextKey) -> &'static str {
    match locale {
        Locale::En => match key {
            TextKey::HeaderTitle => " Dead By Queue ",
            TextKey::SortLabel => "Sort: ",
            TextKey::ModeLabel => "Mode: ",
            TextKey::LockLabel => "Lock: ",
            TextKey::LockNone => "None",
            TextKey::LockActive => "Active",
            TextKey::ColRegion => "Region",
            TextKey::ColPing => "Ping",
            TextKey::ColSurvivor => "Survivor",
            TextKey::ColKiller => "Killer",
            TextKey::FetchingQueues => "  Fetching queue times...",
            TextKey::FailedQueues => "  Failed to load queue data. Check network or proxy. Press 'R' to retry.",
            TextKey::NoDataForMode => "  No queue data available for this mode.",
            TextKey::ActionSelect => "Select ",
            TextKey::ActionLock => "Lock ",
            TextKey::ActionSort => "Sort ",
            TextKey::ActionMode => "Mode ",
            TextKey::ActionRefresh => "Refresh ",
            TextKey::ActionQuit => "Quit ",
            TextKey::TimeFetching => "fetching...",
            TextKey::TimeJustNow => "just now",
            TextKey::StatusFetching => "Fetching...",
            TextKey::StatusApiUpdated => "API Updated: ",
            TextKey::FeedbackUpToDate => "[✓ Up to date]",
            TextKey::FeedbackUpdated => "[✓ Updated]",
            TextKey::ErrorPrefix => "Error: ",
            TextKey::ErrorConfigSave => "Failed to save config",
            TextKey::ModalTitle => " Region Locker ",
            TextKey::ModalSelect => "Select",
            TextKey::ModalToggle => "Toggle",
            TextKey::ModalApply => "Apply",
            TextKey::ModalCancel => "Cancel",
            TextKey::HostsUpdated => "Region locks updated!",
            TextKey::HostsAlreadyUpToDate => "Hosts file is up to date",
            TextKey::HostsElevationFailed => "Error: admin elevation denied",
        },
        Locale::Ru => match key {
            TextKey::HeaderTitle => " Dead By Queue ",
            TextKey::SortLabel => "Сортировка: ",
            TextKey::ModeLabel => "Режим: ",
            TextKey::LockLabel => "Блокировка: ",
            TextKey::LockNone => "Все",
            TextKey::LockActive => "Активен",
            TextKey::ColRegion => "Регион",
            TextKey::ColPing => "Пинг",
            TextKey::ColSurvivor => "Выживший",
            TextKey::ColKiller => "Маньяк",
            TextKey::FetchingQueues => "  Загрузка данных очередей...",
            TextKey::FailedQueues => "  Не удалось загрузить данные очередей. Проверьте сеть или прокси. Нажмите 'R' для повтора.",
            TextKey::NoDataForMode => "  Нет данных для выбранного режима.",
            TextKey::ActionSelect => "Выбор ",
            TextKey::ActionLock => "Блокировка ",
            TextKey::ActionSort => "Сортировка ",
            TextKey::ActionMode => "Режим ",
            TextKey::ActionRefresh => "Обновить ",
            TextKey::ActionQuit => "Выход ",
            TextKey::TimeFetching => "Загрузка...",
            TextKey::TimeJustNow => "только что",
            TextKey::StatusFetching => "Обновление...",
            TextKey::StatusApiUpdated => "API обновлено: ",
            TextKey::FeedbackUpToDate => "[✓ Актуально]",
            TextKey::FeedbackUpdated => "[✓ Обновлено]",
            TextKey::ErrorPrefix => "Ошибка: ",
            TextKey::ErrorConfigSave => "Не удалось сохранить конфигурацию",
            TextKey::ModalTitle => " Блокировка регионов ",
            TextKey::ModalSelect => "Выбор",
            TextKey::ModalToggle => "Вкл/Выкл",
            TextKey::ModalApply => "Применить",
            TextKey::ModalCancel => "Отмена",
            TextKey::HostsUpdated => "Блокировка регионов обновлена!",
            TextKey::HostsAlreadyUpToDate => "Файл hosts уже актуален",
            TextKey::HostsElevationFailed => "Ошибка: отказ в правах Администратора",
        },
    }
}

pub fn tr_sort(locale: Locale, sort: SortOrder) -> &'static str {
    match locale {
        Locale::En => match sort {
            SortOrder::Default => "Default",
            SortOrder::Killer => "Killer",
            SortOrder::Survivor => "Survivor",
            SortOrder::Ping => "Ping",
        },
        Locale::Ru => match sort {
            SortOrder::Default => "По умолчанию",
            SortOrder::Killer => "Маньяк",
            SortOrder::Survivor => "Выживший",
            SortOrder::Ping => "Пинг",
        },
    }
}

pub fn tr_mode(locale: Locale, mode: GameMode) -> &'static str {
    match locale {
        Locale::En => match mode {
            GameMode::Standard => "Standard",
            GameMode::Event => "Event",
            GameMode::Both => "Both",
        },
        Locale::Ru => match mode {
            GameMode::Standard => "Обычный",
            GameMode::Event => "Ивент",
            GameMode::Both => "Оба",
        },
    }
}

pub fn format_time_diff(locale: Locale, diff_secs: i64) -> String {
    if diff_secs < 60 {
        tr(locale, TextKey::TimeJustNow).to_string()
    } else if diff_secs < 3600 {
        let mins = diff_secs / 60;
        match locale {
            Locale::En => format!("{}m ago", mins),
            Locale::Ru => format!("{} мин. назад", mins),
        }
    } else {
        let hours = diff_secs / 3600;
        let mins = (diff_secs % 3600) / 60;
        match locale {
            Locale::En => format!("{}h {}m ago", hours, mins),
            Locale::Ru => format!("{} ч. {} мин. назад", hours, mins),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_precedence() {
        // Explicit En
        assert_eq!(
            resolve_locale_from_sources(Language::En, Some("ru_RU"), Some("ru"), Some("ru"), Some("ru")),
            Locale::En
        );
        // Explicit Ru
        assert_eq!(
            resolve_locale_from_sources(Language::Ru, Some("en_US"), Some("en"), Some("en"), Some("en")),
            Locale::Ru
        );

        // Auto: sys_locale takes precedence
        assert_eq!(
            resolve_locale_from_sources(Language::Auto, Some("ru-RU"), Some("en_US"), Some("en_US"), Some("en_US")),
            Locale::Ru
        );
        assert_eq!(
            resolve_locale_from_sources(Language::Auto, Some("en-US"), Some("ru_RU"), Some("ru_RU"), Some("ru_RU")),
            Locale::En
        );

        // Auto: sys_locale empty / None -> falls back to LC_ALL
        assert_eq!(
            resolve_locale_from_sources(Language::Auto, None, Some("ru_RU.UTF-8"), Some("en"), Some("en")),
            Locale::Ru
        );
        assert_eq!(
            resolve_locale_from_sources(Language::Auto, Some(""), Some("en_US.UTF-8"), Some("ru"), Some("ru")),
            Locale::En
        );

        // Auto: LC_ALL empty / None -> falls back to LC_MESSAGES
        assert_eq!(
            resolve_locale_from_sources(Language::Auto, None, None, Some("ru_RU"), Some("en")),
            Locale::Ru
        );
        assert_eq!(
            resolve_locale_from_sources(Language::Auto, None, Some("   "), Some("en_US"), Some("ru")),
            Locale::En
        );

        // Auto: LC_MESSAGES empty / None -> falls back to LANG
        assert_eq!(
            resolve_locale_from_sources(Language::Auto, None, None, None, Some("ru_RU.UTF-8")),
            Locale::Ru
        );
        assert_eq!(
            resolve_locale_from_sources(Language::Auto, None, None, None, Some("fr_FR")),
            Locale::En
        );

        // Auto: All empty -> defaults to En
        assert_eq!(
            resolve_locale_from_sources(Language::Auto, None, None, None, None),
            Locale::En
        );
    }
}
