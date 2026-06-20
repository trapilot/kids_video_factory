use rand::Rng;

use crate::enums::*;

pub fn start_of_today_ts() -> i64 {
    let now = chrono::Utc::now();
    let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
    start.and_utc().timestamp()
}

pub fn start_of_recent_ts(days: u8) -> i64 {
    let days_back = days.saturating_sub(1) as u64;
    chrono::Utc::now()
        .date_naive()
        .checked_sub_days(chrono::Days::new(days_back))
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp()
}

pub fn next_backoff(current_ms: u64) -> u64 {
    let max_backoff = if current_ms == 0 {
        60_000
    } else {
        (current_ms * 2).min(30 * 60_000)
    };

    rand::thread_rng().gen_range(0..=max_backoff)
}

pub fn detect_media(bytes: &[u8]) -> Option<MediaType> {
    if is_html(bytes) {
        return None;
    }

    if is_png(bytes) || is_jpg(bytes) || is_webp(bytes) {
        return Some(MediaType::Image);
    }

    if is_mp4(bytes) {
        return Some(MediaType::Video);
    }

    if is_mp3(bytes) {
        return Some(MediaType::Audio);
    }

    None
}

pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G'])
}

pub fn is_jpg(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
}

pub fn is_webp(bytes: &[u8]) -> bool {
    bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP"
}

pub fn is_mp4(bytes: &[u8]) -> bool {
    bytes.len() > 12 &&
    bytes[4..8] == [b'f', b't', b'y', b'p']
}

pub fn is_mp3(bytes: &[u8]) -> bool {
    bytes.starts_with(b"ID3") || bytes[0] == 0xFF
}

pub fn is_html(bytes: &[u8]) -> bool {
    bytes.starts_with(b"<!DOCTYPE")
        || bytes.starts_with(b"<html")
        || bytes.starts_with(b"<HTML")
}