use std::time::Duration;

use crate::error::RescopeError;

pub const MINIMUM_INTERVAL: Duration = Duration::from_millis(250);

pub fn parse_duration(input: &str) -> Result<Duration, RescopeError> {
    humantime::parse_duration(input).map_err(|_| RescopeError::InvalidDuration {
        input: input.to_string(),
    })
}

pub fn format_bytes(bytes: u64, raw_bytes: bool) -> String {
    if raw_bytes {
        return format!("{bytes} B");
    }

    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut value = bytes as f64;
    let mut unit_index = 0;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{bytes} B")
    } else if value >= 10.0 {
        format!("{value:.0} {}", UNITS[unit_index])
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}

pub fn format_signed_bytes(bytes: i64, raw_bytes: bool) -> String {
    let sign = if bytes >= 0 { "+" } else { "-" };
    let absolute = bytes.unsigned_abs();
    format!("{sign}{}", format_bytes(absolute, raw_bytes))
}

pub fn format_bps(bytes_per_second: f64, raw_bytes: bool) -> String {
    if raw_bytes {
        return format!("{bytes_per_second:.0} B/s");
    }

    if bytes_per_second <= 0.0 {
        "0 B/s".to_string()
    } else {
        format!("{}/s", format_bytes(bytes_per_second.round() as u64, false))
    }
}

pub fn sparkline(values: &[u64], width: usize) -> String {
    const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    if values.is_empty() || width == 0 {
        return String::new();
    }

    let compressed = compress_values(values, width);
    let min = compressed.iter().min().copied().unwrap_or(0);
    let max = compressed.iter().max().copied().unwrap_or(0);

    if min == max {
        return std::iter::repeat_n('▁', compressed.len()).collect();
    }

    let range = (max - min) as f64;
    compressed
        .iter()
        .map(|value| {
            let normalized = (*value - min) as f64 / range;
            let index = (normalized * (BLOCKS.len() - 1) as f64).round() as usize;
            BLOCKS[index.min(BLOCKS.len() - 1)]
        })
        .collect()
}

fn compress_values(values: &[u64], width: usize) -> Vec<u64> {
    if values.len() <= width {
        return values.to_vec();
    }

    let chunk_size = values.len() as f64 / width as f64;
    (0..width)
        .map(|index| {
            let start = (index as f64 * chunk_size).floor() as usize;
            let end = (((index + 1) as f64 * chunk_size).ceil() as usize).min(values.len());
            let slice = &values[start..end.max(start + 1).min(values.len())];
            let sum: u128 = slice.iter().map(|value| *value as u128).sum();
            (sum / slice.len() as u128) as u64
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_human_durations() {
        assert_eq!(parse_duration("250ms").unwrap(), Duration::from_millis(250));
        assert_eq!(parse_duration("1m").unwrap(), Duration::from_secs(60));
        assert!(parse_duration("abc").is_err());
    }

    #[test]
    fn formats_bytes() {
        assert_eq!(format_bytes(0, false), "0 B");
        assert_eq!(format_bytes(1024, false), "1.0 KiB");
        assert_eq!(format_bytes(1_048_576, false), "1.0 MiB");
        assert_eq!(format_bytes(42, true), "42 B");
    }

    #[test]
    fn creates_sparkline() {
        assert_eq!(sparkline(&[1, 1, 1], 10), "▁▁▁");
        let line = sparkline(&[1, 2, 3, 4, 5], 3);
        assert_eq!(line.chars().count(), 3);
    }
}
