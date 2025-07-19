use crate::model::{StringStat, take_a_char_off};
use chrono::DateTime;

pub fn process_tee_time(tee_time: &str) -> Option<StringStat> {
    let mut_tee_time = if tee_time.ends_with("Z") {
        format!("{tee_time}+0000")
    } else {
        tee_time.to_owned()
    };

    let mut failed_to_parse = false;
    let parsed_time = match DateTime::parse_from_str(&mut_tee_time, "%Y-%m-%dT%H:%MZ%z") {
        Ok(dt) => dt,
        Err(_e) => {
            failed_to_parse = true;
            DateTime::parse_from_rfc3339("2000-01-01T00:00:00+00:00")
                .expect("Hardcoded fallback date should always be valid")
        }
    };

    let central_timezone = chrono::offset::FixedOffset::east_opt(-5 * 3600).unwrap_or_else(|| {
        chrono::offset::FixedOffset::east_opt(0).expect("UTC timezone offset is always valid")
    });

    let parsed_time_in_central = parsed_time.with_timezone(&central_timezone);

    let special_format_time =
        take_a_char_off(&parsed_time_in_central.format("%-m/%d %-I:%M%P").to_string()).to_string();

    if !failed_to_parse {
        Some(StringStat {
            val: special_format_time,
        })
    } else {
        None
    }
}
