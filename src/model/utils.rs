use chrono::Duration as ChronoDuration;

pub fn format_time_ago_for_score_view(td: ChronoDuration) -> String {
    let secs = td.num_seconds();

    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    const WEEK: i64 = 7 * DAY;
    const MONTH: i64 = 30 * DAY;
    const YEAR: i64 = 365 * DAY;

    if secs >= YEAR {
        let years = (secs as f64) / (YEAR as f64);
        if years == 1.0 {
            "1 year".to_string()
        } else {
            format!("{:.2} years", years)
        }
    } else if secs >= MONTH {
        let months = (secs as f64) / (MONTH as f64);
        format!("{:.2} months", months)
    } else if secs >= WEEK {
        let weeks = secs / WEEK;
        if weeks == 1 {
            "1 week".to_string()
        } else {
            format!("{} weeks", weeks)
        }
    } else if secs >= DAY {
        let days = secs / DAY;
        if days == 1 {
            "1 day".to_string()
        } else {
            format!("{} days", days)
        }
    } else if secs >= HOUR {
        let hours = secs / HOUR;
        if hours == 1 {
            "1 hour".to_string()
        } else {
            format!("{} hours", hours)
        }
    } else if secs >= MINUTE {
        let minutes = secs / MINUTE;
        if minutes == 1 {
            "1 minute".to_string()
        } else {
            format!("{} minutes", minutes)
        }
    } else {
        if secs == 1 {
            "1 second".to_string()
        } else {
            format!("{} seconds", secs)
        }
    }
}

pub fn take_a_char_off(s: &str) -> String {
    let mut result = s.to_string();
    result.pop();
    result
}