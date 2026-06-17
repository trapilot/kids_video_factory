use chrono::{Local, Datelike, Utc};
use rand::Rng;

use crate::enums::*;

pub fn now_ts() -> String {
    Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub fn now_ymd() -> String {
    Local::now()
        .format("%Y%m%d")
        .to_string()
}

pub fn now_rfc() -> String {
    Local::now().to_rfc3339()
}

pub fn now_age(birth_year: i32) -> i32 {
    calc_age(birth_year, Utc::now().year())
}

pub fn calc_age(birth_year: i32, current_year: i32) -> i32 {
    current_year - birth_year
}

pub fn left_pad(num: u32, digits: usize) -> String {
    format!("{:0width$}", num, width = digits)
}

pub fn next_backoff(current_ms: u64) -> u64 {
    let max_backoff = if current_ms == 0 {
        60_000
    } else {
        (current_ms * 2).min(30 * 60_000)
    };

    rand::thread_rng().gen_range(0..=max_backoff)
}

pub fn ctx(node: &str, err: impl ToString) -> String {
    format!("[{}] {}", node, err.to_string())
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

pub fn is_retryable_error(msg: &str) -> bool {
    msg.contains("503")
        || msg.contains("UNAVAILABLE")
        || msg.contains("429")
        || msg.contains("rate")
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