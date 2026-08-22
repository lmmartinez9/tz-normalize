use std::fmt;

#[derive(Debug, PartialEq)]
pub struct Date {
    pub year: u32,
    pub month: u32,
    pub day: u32,
}

#[derive(Debug, PartialEq)]
pub struct Time {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

#[derive(Debug, PartialEq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Option<Time>,
    pub offset_minutes: Option<i32>,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Empty,
    BadDate(String),
    BadTime(String),
    BadOffset(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty line"),
            ParseError::BadDate(s) => write!(f, "unrecognized date: {:?}", s),
            ParseError::BadTime(s) => write!(f, "unrecognized time: {:?}", s),
            ParseError::BadOffset(s) => write!(f, "unrecognized offset: {:?}", s),
        }
    }
}

/// Parses one messy timestamp line and rewrites it as `YYYY-MM-DD[THH:MM:SS[+HH:MM|Z]]`.
pub fn normalize_line(line: &str) -> Result<String, ParseError> {
    let ts = parse_timestamp(line)?;
    Ok(format_timestamp(&ts))
}

fn parse_timestamp(line: &str) -> Result<Timestamp, ParseError> {
    let s = line.trim();
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    // The date and time portions are split on the first 'T' or run of
    // whitespace; everything before that has to be the date. Month-name
    // dates ("Jan 5 2024") span more than one word, so if the numeric
    // parser rejects that first word, fall back to scanning the whole
    // line for a month name instead.
    let split_idx = s.find(|c: char| c == 'T' || c.is_whitespace());
    let (date_part, rest) = match split_idx {
        Some(i) => (&s[..i], s[i + 1..].trim_start()),
        None => (s, ""),
    };

    let (date, rest) = match parse_date(date_part) {
        Ok(date) => (date, rest),
        Err(_) => parse_month_name_date(s)?,
    };

    if rest.is_empty() {
        return Ok(Timestamp {
            date,
            time: None,
            offset_minutes: None,
        });
    }

    // A 24-hour clock only ever contains digits and colons, so the first
    // sign or zone letter (numeric offset or named abbreviation) marks the
    // start of the offset.
    let offset_idx = rest.find(|c: char| matches!(c, 'Z' | 'z' | '+' | '-') || c.is_ascii_alphabetic());
    let (time_part, offset_part) = match offset_idx {
        Some(i) => (rest[..i].trim(), Some(rest[i..].trim())),
        None => (rest.trim(), None),
    };

    let time = parse_time(time_part)?;
    let offset_minutes = match offset_part {
        Some(o) => Some(parse_offset(o)?),
        None => None,
    };

    Ok(Timestamp {
        date,
        time: Some(time),
        offset_minutes,
    })
}

fn parse_date(s: &str) -> Result<Date, ParseError> {
    let sep = if s.contains('-') {
        '-'
    } else if s.contains('/') {
        '/'
    } else {
        return Err(ParseError::BadDate(s.to_string()));
    };

    let parts: Vec<&str> = s.split(sep).collect();
    if parts.len() != 3 {
        return Err(ParseError::BadDate(s.to_string()));
    }

    let year = parts[0]
        .parse::<u32>()
        .map_err(|_| ParseError::BadDate(s.to_string()))?;
    let month = parts[1]
        .parse::<u32>()
        .map_err(|_| ParseError::BadDate(s.to_string()))?;
    let day = parts[2]
        .parse::<u32>()
        .map_err(|_| ParseError::BadDate(s.to_string()))?;

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(ParseError::BadDate(s.to_string()));
    }

    Ok(Date { year, month, day })
}

/// Splits a leading whitespace-delimited word off `s`, returning it and the
/// (trimmed-at-the-front) remainder.
fn take_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], &s[i + 1..]),
        None => (s, ""),
    }
}

fn month_number(word: &str) -> Option<u32> {
    let n = match word.to_ascii_lowercase().as_str() {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "sept" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return None,
    };
    Some(n)
}

/// Parses a date spelled with a month name at the start of `s`, in either
/// "Month Day Year" or "Day Month Year" order, an optional comma after the
/// day, and returns the leftover of `s` (the time portion, if any).
fn parse_month_name_date(s: &str) -> Result<(Date, &str), ParseError> {
    let (w1, r1) = take_word(s);
    let (w2, r2) = take_word(r1);
    let (w3, r3) = take_word(r2);

    let attempt = |month: Option<u32>, day_word: &str, year_word: &str| -> Option<Date> {
        let month = month?;
        let day = day_word.trim_end_matches(',').parse::<u32>().ok()?;
        let year = year_word.parse::<u32>().ok()?;
        if (1..=31).contains(&day) {
            Some(Date { year, month, day })
        } else {
            None
        }
    };

    if let Some(date) = attempt(month_number(w1), w2, w3) {
        return Ok((date, r3));
    }
    if let Some(date) = attempt(month_number(w2), w1, w3) {
        return Ok((date, r3));
    }

    Err(ParseError::BadDate(s.to_string()))
}

fn parse_time(s: &str) -> Result<Time, ParseError> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Err(ParseError::BadTime(s.to_string()));
    }

    let hour = parts[0]
        .parse::<u32>()
        .map_err(|_| ParseError::BadTime(s.to_string()))?;
    let minute = parts[1]
        .parse::<u32>()
        .map_err(|_| ParseError::BadTime(s.to_string()))?;
    let second = if parts.len() == 3 {
        parts[2]
            .parse::<u32>()
            .map_err(|_| ParseError::BadTime(s.to_string()))?
    } else {
        0
    };

    if hour > 23 || minute > 59 || second > 59 {
        return Err(ParseError::BadTime(s.to_string()));
    }

    Ok(Time {
        hour,
        minute,
        second,
    })
}

/// Fixed UTC offset, in minutes, for common zone abbreviations. These are
/// not real timezones (no DST rules, no political history) - just the
/// single offset each abbreviation is conventionally used to mean. Where an
/// abbreviation is genuinely ambiguous (AST, IST, ...) this picks the most
/// common reading rather than trying to guess from context.
fn zone_offset_minutes(name: &str) -> Option<i32> {
    let minutes = match name {
        "UTC" | "GMT" | "WET" => 0,
        "BST" | "CET" | "WEST" => 60,
        "CEST" | "EET" => 120,
        "EEST" | "MSK" => 180,
        "IST" => 330,
        "JST" | "KST" => 540,
        "AWST" => 480,
        "ACST" => 570,
        "ACDT" => 630,
        "AEST" => 600,
        "AEDT" => 660,
        "NZST" => 720,
        "NZDT" => 780,
        "NST" => -210,
        "AST" | "EDT" => -240,
        "EST" | "CDT" => -300,
        "CST" | "MDT" => -360,
        "MST" | "PDT" => -420,
        "PST" => -480,
        _ => return None,
    };
    Some(minutes)
}

fn parse_offset(s: &str) -> Result<i32, ParseError> {
    if s.eq_ignore_ascii_case("z") {
        return Ok(0);
    }

    if s.chars().all(|c| c.is_ascii_alphabetic()) {
        return zone_offset_minutes(&s.to_ascii_uppercase())
            .ok_or_else(|| ParseError::BadOffset(s.to_string()));
    }

    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return Err(ParseError::BadOffset(s.to_string()));
    }

    let sign = match bytes[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return Err(ParseError::BadOffset(s.to_string())),
    };

    let body = &s[1..];
    if body.is_empty() {
        return Err(ParseError::BadOffset(s.to_string()));
    }

    let (hour_str, minute_str): (&str, &str) = if let Some(pos) = body.find(':') {
        (&body[..pos], &body[pos + 1..])
    } else {
        match body.len() {
            4 => (&body[0..2], &body[2..4]),
            2 => (&body[0..2], "0"),
            1 => (&body[0..1], "0"),
            _ => return Err(ParseError::BadOffset(s.to_string())),
        }
    };

    let hour = hour_str
        .parse::<i32>()
        .map_err(|_| ParseError::BadOffset(s.to_string()))?;
    let minute = minute_str
        .parse::<i32>()
        .map_err(|_| ParseError::BadOffset(s.to_string()))?;

    if hour > 14 || minute > 59 {
        return Err(ParseError::BadOffset(s.to_string()));
    }

    Ok(sign * (hour * 60 + minute))
}

fn format_timestamp(ts: &Timestamp) -> String {
    let mut out = format!("{:04}-{:02}-{:02}", ts.date.year, ts.date.month, ts.date.day);

    if let Some(time) = &ts.time {
        out.push('T');
        out.push_str(&format!(
            "{:02}:{:02}:{:02}",
            time.hour, time.minute, time.second
        ));

        if let Some(off) = ts.offset_minutes {
            if off == 0 {
                out.push('Z');
            } else {
                let sign = if off < 0 { '-' } else { '+' };
                let abs = off.abs();
                out.push_str(&format!("{}{:02}:{:02}", sign, abs / 60, abs % 60));
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_only_gets_zero_padded() {
        assert_eq!(normalize_line("2024-1-5").unwrap(), "2024-01-05");
    }

    #[test]
    fn slash_separated_date() {
        assert_eq!(normalize_line("2024/1/5").unwrap(), "2024-01-05");
    }

    #[test]
    fn zulu_offset() {
        assert_eq!(
            normalize_line("2024-01-05T09:30:00Z").unwrap(),
            "2024-01-05T09:30:00Z"
        );
    }

    #[test]
    fn space_separated_with_colon_offset() {
        assert_eq!(
            normalize_line("2024-01-05 09:30:00 -05:00").unwrap(),
            "2024-01-05T09:30:00-05:00"
        );
    }

    #[test]
    fn compact_offset_without_colon() {
        assert_eq!(
            normalize_line("2024-01-05T09:30:00+0530").unwrap(),
            "2024-01-05T09:30:00+05:30"
        );
    }

    #[test]
    fn missing_seconds_default_to_zero() {
        assert_eq!(
            normalize_line("2024-01-05T09:30Z").unwrap(),
            "2024-01-05T09:30:00Z"
        );
    }

    #[test]
    fn empty_line_is_an_error() {
        assert_eq!(normalize_line("   "), Err(ParseError::Empty));
    }

    #[test]
    fn unrecognized_date_is_an_error() {
        assert!(normalize_line("Nowhere 5 2024").is_err());
    }

    #[test]
    fn month_name_then_day_and_year() {
        assert_eq!(normalize_line("Jan 5 2024").unwrap(), "2024-01-05");
    }

    #[test]
    fn day_then_month_name_and_year() {
        assert_eq!(normalize_line("5 January 2024").unwrap(), "2024-01-05");
    }

    #[test]
    fn month_name_date_with_comma_after_day() {
        assert_eq!(normalize_line("January 5, 2024").unwrap(), "2024-01-05");
    }

    #[test]
    fn month_name_is_case_insensitive() {
        assert_eq!(normalize_line("jan 5 2024").unwrap(), "2024-01-05");
    }

    #[test]
    fn month_name_date_with_time_and_offset() {
        assert_eq!(
            normalize_line("Jan 5 2024 09:30:00Z").unwrap(),
            "2024-01-05T09:30:00Z"
        );
    }

    #[test]
    fn unrecognized_month_name_is_an_error() {
        assert!(normalize_line("Foo 5 2024").is_err());
    }

    #[test]
    fn zone_abbreviation_is_converted_to_fixed_offset() {
        assert_eq!(
            normalize_line("2024-01-05T09:30:00 EST").unwrap(),
            "2024-01-05T09:30:00-05:00"
        );
    }

    #[test]
    fn zone_abbreviation_without_separating_space() {
        assert_eq!(
            normalize_line("2024-01-05T09:30:00PST").unwrap(),
            "2024-01-05T09:30:00-08:00"
        );
    }

    #[test]
    fn zone_abbreviation_is_case_insensitive() {
        assert_eq!(
            normalize_line("2024-01-05T09:30:00 jst").unwrap(),
            "2024-01-05T09:30:00+09:00"
        );
    }

    #[test]
    fn half_hour_zone_abbreviation() {
        assert_eq!(
            normalize_line("2024-01-05T09:30:00 IST").unwrap(),
            "2024-01-05T09:30:00+05:30"
        );
    }

    #[test]
    fn unknown_zone_abbreviation_is_an_error() {
        assert!(normalize_line("2024-01-05T09:30:00 XYZ").is_err());
    }
}
