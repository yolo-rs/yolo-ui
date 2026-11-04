// SPDX-License-Identifier: Apache-2.0
//! Diagnostics with file:line:column positions. Every unsupported construct becomes an error;
//! the migrator never writes output while an error is pending.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Diag {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub msg: String,
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}:{}:{}: {}", self.file, self.line, self.col, self.msg)
    }
}

impl Diag {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"severity\":\"error\",\"file\":{},\"line\":{},\"column\":{},\"message\":{}}}",
            json_str(&self.file),
            self.line,
            self.col,
            json_str(&self.msg)
        )
    }
}

pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Maps byte offsets to 1-based line and column (columns count characters).
pub struct LineIndex<'s> {
    src: &'s str,
    starts: Vec<usize>,
}

impl<'s> LineIndex<'s> {
    pub fn new(src: &'s str) -> Self {
        let mut starts = vec![0];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        Self { src, starts }
    }

    pub fn pos(&self, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(self.src.len());
        let line = match self.starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let start = self.starts[line];
        let col = self.src.get(start..offset).map_or(offset - start, |s| s.chars().count());
        (line as u32 + 1, col as u32 + 1)
    }
}
