use chrono::{DateTime, Local, TimeZone, Timelike};
use chrono_tz::Tz;

pub(super) fn format_event_date_local(value: &str) -> Option<String> {
    let parsed = DateTime::parse_from_rfc3339(value).ok()?;
    if let Some(tz) = resolve_tz() {
        let local = parsed.with_timezone(&tz);
        return Some(format_event_date(&local));
    }
    let local = parsed.with_timezone(&Local);
    Some(format_event_date(&local))
}

fn resolve_tz() -> Option<Tz> {
    std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse::<Tz>().ok())
}

fn format_event_date<TzType>(local: &DateTime<TzType>) -> String
where
    TzType: TimeZone,
    TzType::Offset: std::fmt::Display,
{
    let date = local.format("%b %-d %Y").to_string();
    let (is_pm, hour12) = local.time().hour12();
    let suffix = if is_pm { "p" } else { "a" };
    let tz_abbr = local.format("%Z").to_string();
    format!(
        "{}, {}:{:02}{} {}",
        date,
        hour12,
        local.minute(),
        suffix,
        tz_abbr
    )
}
