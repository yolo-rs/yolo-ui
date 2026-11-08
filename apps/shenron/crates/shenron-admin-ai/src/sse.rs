//! Incremental Server-Sent Events parser (WHATWG `text/event-stream`), used by the native
//! client; the browser uses its own `EventSource`.

/// One dispatched event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SseEvent {
    pub event: String,
    pub data: String,
}

#[derive(Default, Debug)]
pub struct SseParser {
    bytes: Vec<u8>,
    buf: String,
    event: String,
    data: Vec<String>,
}

impl SseParser {
    /// Feeds raw bytes; a multi-byte character split across chunks is kept until it completes
    /// (only whole lines, which end on an ASCII newline, are decoded).
    pub fn push_bytes(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.bytes.extend_from_slice(chunk);
        let Some(last) = self.bytes.iter().rposition(|b| *b == b'\n') else {
            return Vec::new();
        };
        let lines: Vec<u8> = self.bytes.drain(..=last).collect();
        self.push(&String::from_utf8_lossy(&lines))
    }

    /// Feeds a chunk; returns the events completed by it.
    pub fn push(&mut self, chunk: &str) -> Vec<SseEvent> {
        self.buf.push_str(chunk);
        let mut out = Vec::new();
        while let Some(pos) = self.buf.find('\n') {
            let mut line: String = self.buf.drain(..=pos).collect();
            line.pop();
            if line.ends_with('\r') {
                line.pop();
            }
            if line.is_empty() {
                if !self.data.is_empty() {
                    let event = if self.event.is_empty() {
                        "message".to_owned()
                    } else {
                        std::mem::take(&mut self.event)
                    };
                    out.push(SseEvent { event, data: self.data.join("\n") });
                }
                self.data.clear();
                self.event.clear();
                continue;
            }
            if line.starts_with(':') {
                continue; // comment / keep-alive
            }
            let (field, value) = match line.find(':') {
                Some(i) => {
                    let v = &line[i + 1..];
                    (&line[..i], v.strip_prefix(' ').unwrap_or(v))
                }
                None => (line.as_str(), ""),
            };
            match field {
                "event" => self.event = value.to_owned(),
                "data" => self.data.push(value.to_owned()),
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_split_across_chunks_survives() {
        let mut p = SseParser::default();
        let msg = "data: かめはめ波\n\n".as_bytes();
        let (a, b) = msg.split_at(8);
        assert!(p.push_bytes(a).is_empty());
        assert_eq!(p.push_bytes(b)[0].data, "かめはめ波");
    }

    #[test]
    fn parses_across_chunks_comments_and_crlf() {
        let mut p = SseParser::default();
        assert!(p.push("event: hel").is_empty());
        assert!(p.push("lo\r\ndata: {\"a\":1}\r\n").is_empty());
        let ev = p.push("\r\n:keep-alive\n\ndata: x\ndata: y\n\n");
        assert_eq!(
            ev,
            vec![
                SseEvent { event: "hello".into(), data: "{\"a\":1}".into() },
                SseEvent { event: "message".into(), data: "x\ny".into() },
            ]
        );
    }
}
