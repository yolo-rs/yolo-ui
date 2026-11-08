//! Locale-aware formatting and the pure view helpers ported from `ia-cockpit-view.ts`.

use crate::i18n::Lang;

/// `fmtInt`: grouped integer (fr: narrow no-break space, en/ja: comma); `—` when absent.
pub fn int(lang: Lang, value: Option<u64>) -> String {
    match value {
        Some(v) => group(lang, &v.to_string()),
        None => "—".to_owned(),
    }
}

fn group(lang: Lang, digits: &str) -> String {
    let sep = match lang {
        Lang::Fr => '\u{202f}',
        Lang::En | Lang::Ja => ',',
    };
    let mut out = String::new();
    let n = digits.len();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (n - i).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(ch);
    }
    out
}

/// Decimal with `places` digits and the locale separator.
pub fn decimal(lang: Lang, value: f64, places: usize) -> String {
    let s = format!("{value:.places$}");
    match lang {
        Lang::Fr => s.replace('.', ","),
        Lang::En | Lang::Ja => s,
    }
}

/// `fmtPct`: ratio in `[0, 1]` as a percentage with one decimal; `—` when absent.
pub fn pct(lang: Lang, value: Option<f64>) -> String {
    match value {
        Some(v) if v.is_finite() => {
            let body = decimal(lang, v * 100.0, 1);
            match lang {
                Lang::Fr => format!("{body}\u{202f}%"),
                Lang::En | Lang::Ja => format!("{body}%"),
            }
        }
        _ => "—".to_owned(),
    }
}

/// Milliseconds, rounded.
pub fn ms(lang: Lang, value: f64) -> String {
    format!("{}\u{a0}ms", int(lang, Some(value.max(0.0).round() as u64)))
}

/// A parsed UTC instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instant {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// Days since 1970-01-01 of a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

impl Instant {
    /// Parses `YYYY-MM-DDTHH:MM:SS[.fff](Z|±HH:MM)`; the result is in UTC.
    pub fn parse(iso: &str) -> Option<Instant> {
        let b = iso.as_bytes();
        if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || (b[10] != b'T' && b[10] != b' ') {
            return None;
        }
        let num = |r: std::ops::Range<usize>| iso.get(r)?.parse::<i64>().ok();
        let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
        let (h, mi, s) = (num(11..13)?, num(14..16)?, num(17..19)?);
        if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
            return None;
        }
        let mut rest = &iso[19..];
        if let Some(stripped) = rest.strip_prefix('.') {
            let frac = stripped
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(stripped.len());
            rest = &stripped[frac..];
        }
        let offset_min: i64 = match rest {
            "" | "Z" | "z" => 0,
            r if (r.starts_with('+') || r.starts_with('-')) && r.len() == 6 => {
                let sign = if r.starts_with('-') { -1 } else { 1 };
                let oh = r.get(1..3)?.parse::<i64>().ok()?;
                let om = r.get(4..6)?.parse::<i64>().ok()?;
                sign * (oh * 60 + om)
            }
            _ => return None,
        };
        let secs = days_from_civil(y, mo as u32, d as u32) * 86_400 + h * 3600 + mi * 60 + s
            - offset_min * 60;
        Some(Instant::from_unix(secs))
    }

    pub fn from_unix(secs: i64) -> Instant {
        let days = secs.div_euclid(86_400);
        let rem = secs.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        Instant {
            year,
            month,
            day,
            hour: (rem / 3600) as u32,
            minute: ((rem % 3600) / 60) as u32,
            second: (rem % 60) as u32,
        }
    }

    pub fn unix(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day) * 86_400
            + self.hour as i64 * 3600
            + self.minute as i64 * 60
            + self.second as i64
    }

    /// Shifted by a UTC offset in minutes (local time display).
    pub fn shifted(&self, offset_min: i32) -> Instant {
        Instant::from_unix(self.unix() + offset_min as i64 * 60)
    }

    /// RFC 3339 UTC with a `Z`.
    pub fn to_iso(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// Current time in RFC 3339 (native targets; the browser uses `Date`).
#[cfg(not(target_arch = "wasm32"))]
pub fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Instant::from_unix(secs).to_iso()
}

/// `fmtDate`: date and time in the reader's locale, `—` when absent or unreadable.
pub fn datetime(lang: Lang, iso: Option<&str>, offset_min: i32) -> String {
    let Some(i) = iso.and_then(Instant::parse) else {
        return "—".to_owned();
    };
    let l = i.shifted(offset_min);
    match lang {
        Lang::Fr => format!(
            "{:02}/{:02}/{:04} {:02}:{:02}",
            l.day, l.month, l.year, l.hour, l.minute
        ),
        Lang::En => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            l.year, l.month, l.day, l.hour, l.minute
        ),
        Lang::Ja => format!(
            "{:04}/{:02}/{:02} {:02}:{:02}",
            l.year, l.month, l.day, l.hour, l.minute
        ),
    }
}

/// Date only (agent key creation).
pub fn date(lang: Lang, iso: Option<&str>, offset_min: i32) -> String {
    let Some(i) = iso.and_then(Instant::parse) else {
        return "—".to_owned();
    };
    let l = i.shifted(offset_min);
    match lang {
        Lang::Fr => format!("{:02}/{:02}/{:04}", l.day, l.month, l.year),
        Lang::En => format!("{:04}-{:02}-{:02}", l.year, l.month, l.day),
        Lang::Ja => format!("{:04}/{:02}/{:02}", l.year, l.month, l.day),
    }
}

/// Time of day with seconds (last update, lab messages).
pub fn time(iso: Option<&str>, offset_min: i32) -> String {
    match iso.and_then(Instant::parse) {
        Some(i) => {
            let l = i.shifted(offset_min);
            format!("{:02}:{:02}:{:02}", l.hour, l.minute, l.second)
        }
        None => "—".to_owned(),
    }
}

/// i18n key of a retrieval mode (`modeLabel`); unknown modes are shown verbatim by the caller.
pub fn mode_key(mode: Option<&str>) -> Option<&'static str> {
    Some(match mode {
        Some("hybrid") => "adminAi.mode.hybrid",
        Some("hybrid+rerank") => "adminAi.mode.hybridRerank",
        Some("lexical") => "adminAi.mode.lexical",
        Some("unavailable") => "adminAi.mode.unavailable",
        None | Some("") => "adminAi.mode.unknown",
        Some(_) => return None,
    })
}

/// One run of an OCR excerpt; `marked` runs were inside `<mark>…</mark>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub marked: bool,
}

/// `splitMarkedExcerpt`: splits on `<mark>` highlights; every other character, any other tag
/// included, stays literal text (OCR output is untrusted and never becomes markup).
pub fn split_marked(excerpt: &str) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut marked = false;
    let lower = excerpt.to_ascii_lowercase();
    let mut i = 0;
    let mut start = 0;
    let push = |out: &mut Vec<Segment>, s: &str, marked: bool| {
        if !s.is_empty() {
            out.push(Segment { text: s.to_owned(), marked });
        }
    };
    while i < excerpt.len() {
        if lower[i..].starts_with("<mark>") {
            push(&mut out, &excerpt[start..i], marked);
            marked = true;
            i += 6;
            start = i;
        } else if lower[i..].starts_with("</mark>") {
            push(&mut out, &excerpt[start..i], marked);
            marked = false;
            i += 7;
            start = i;
        } else {
            i += excerpt[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
    push(&mut out, &excerpt[start..], marked);
    out
}

pub const OCR_PROBE_MAX_BYTES: usize = 6_000_000;

/// Accepted planche formats, sniffed from the bytes (the declared type of a file is not trusted).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Webp,
}

pub fn sniff_image(bytes: &[u8]) -> Option<ImageKind> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some(ImageKind::Png)
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(ImageKind::Jpeg)
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(ImageKind::Webp)
    } else {
        None
    }
}

/// Why a planche cannot go to the OCR probe (`ocrProbeFileError`), as an i18n key, or `None`.
pub fn ocr_file_error(bytes: &[u8]) -> Option<&'static str> {
    if sniff_image(bytes).is_none() {
        Some("adminAi.corpus.ocrBadType")
    } else if bytes.len() > OCR_PROBE_MAX_BYTES {
        Some("adminAi.corpus.ocrTooLarge")
    } else {
        None
    }
}

/// Megabytes with one decimal (OCR size message).
pub fn megabytes(lang: Lang, bytes: usize) -> String {
    decimal(lang, bytes as f64 / 1_000_000.0, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_group_per_locale() {
        assert_eq!(int(Lang::Fr, Some(1_234_567)), "1\u{202f}234\u{202f}567");
        assert_eq!(int(Lang::En, Some(1_234)), "1,234");
        assert_eq!(int(Lang::Ja, Some(12)), "12");
        assert_eq!(int(Lang::Fr, None), "—");
    }

    #[test]
    fn percentages_have_one_decimal() {
        assert_eq!(pct(Lang::Fr, Some(0.8123)), "81,2\u{202f}%");
        assert_eq!(pct(Lang::En, Some(1.0)), "100.0%");
        assert_eq!(pct(Lang::En, None), "—");
        assert_eq!(pct(Lang::En, Some(f64::NAN)), "—");
    }

    #[test]
    fn iso_round_trip_and_offsets() {
        let i = Instant::parse("2026-11-05T14:03:09.123Z").unwrap();
        assert_eq!(i.to_iso(), "2026-11-05T14:03:09Z");
        let j = Instant::parse("2026-11-05T15:03:09+01:00").unwrap();
        assert_eq!(i, j);
        assert_eq!(Instant::from_unix(0).to_iso(), "1970-01-01T00:00:00Z");
        assert_eq!(
            Instant::parse("2024-02-29T23:59:59Z")
                .unwrap()
                .shifted(1)
                .to_iso(),
            "2024-03-01T00:00:59Z"
        );
        assert!(Instant::parse("not a date").is_none());
        assert!(Instant::parse("2026-13-01T00:00:00Z").is_none());
        assert_eq!(
            datetime(Lang::Fr, Some("2026-11-05T14:03:09Z"), 60),
            "05/11/2026 15:03"
        );
        assert_eq!(datetime(Lang::En, None, 0), "—");
        assert_eq!(time(Some("2026-11-05T14:03:09Z"), 0), "14:03:09");
    }

    #[test]
    fn mark_split_never_interprets_other_tags() {
        let s = split_marked("a <MARK>Kame</mark> <b>x</b>");
        assert_eq!(
            s,
            vec![
                Segment { text: "a ".into(), marked: false },
                Segment { text: "Kame".into(), marked: true },
                Segment { text: " <b>x</b>".into(), marked: false },
            ]
        );
        assert_eq!(split_marked("かめ<mark>はめ</mark>波").len(), 3);
        assert!(split_marked("").is_empty());
    }

    #[test]
    fn ocr_probe_checks_bytes_not_names() {
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0];
        assert_eq!(sniff_image(&png), Some(ImageKind::Png));
        assert_eq!(sniff_image(b"RIFF\0\0\0\0WEBPVP8 "), Some(ImageKind::Webp));
        assert_eq!(
            sniff_image(&[0xff, 0xd8, 0xff, 0xe0]),
            Some(ImageKind::Jpeg)
        );
        assert_eq!(ocr_file_error(b"GIF89a"), Some("adminAi.corpus.ocrBadType"));
        let mut big = png.to_vec();
        big.resize(OCR_PROBE_MAX_BYTES + 1, 0);
        assert_eq!(ocr_file_error(&big), Some("adminAi.corpus.ocrTooLarge"));
        assert_eq!(ocr_file_error(&png), None);
        assert_eq!(megabytes(Lang::Fr, 6_500_000), "6,5");
    }

    #[test]
    fn mode_keys() {
        assert_eq!(
            mode_key(Some("hybrid+rerank")),
            Some("adminAi.mode.hybridRerank")
        );
        assert_eq!(mode_key(None), Some("adminAi.mode.unknown"));
        assert_eq!(mode_key(Some("custom")), None);
    }
}
