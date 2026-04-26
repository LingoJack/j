use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::error::LlmError;
use super::types::ChatStreamChunk;

/// SSE stream: wraps a `reqwest::Response` byte stream, yields `ChatStreamChunk`.
///
/// Uses a `Vec<u8>` byte buffer to correctly handle multi-byte UTF-8 characters
/// split across TCP chunks — a common occurrence when the server streams CJK text
/// and the network fragments packets at arbitrary byte boundaries.
pub struct SseStream {
    body: Pin<Box<dyn Stream<Item = Result<Vec<u8>, reqwest::Error>> + Send>>,
    byte_buf: Vec<u8>,
    str_buf: String,
}

impl SseStream {
    pub fn new(response: reqwest::Response) -> Self {
        use futures::StreamExt;
        Self {
            body: Box::pin(response.bytes_stream().map(|r| r.map(|b| b.to_vec()))),
            byte_buf: Vec::new(),
            str_buf: String::new(),
        }
    }
}

impl Stream for SseStream {
    type Item = Result<ChatStreamChunk, LlmError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            // Try to extract a complete SSE event from the string buffer
            if let Some(chunk) = try_parse_event(&mut this.str_buf)? {
                return Poll::Ready(Some(Ok(chunk)));
            }

            // Need more data from the body stream
            match Pin::new(&mut this.body).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    this.byte_buf.extend_from_slice(&bytes);
                    // Convert as many valid UTF-8 bytes as possible from byte_buf → str_buf.
                    // Any trailing incomplete multi-byte sequence stays in byte_buf for the next chunk.
                    flush_utf8(&mut this.byte_buf, &mut this.str_buf)?;
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(LlmError::StreamInterrupted(e.to_string()))));
                }
                Poll::Ready(None) => {
                    // Stream ended. Flush any remaining bytes (truly invalid UTF-8 at this point).
                    if !this.byte_buf.is_empty() {
                        match std::str::from_utf8(&this.byte_buf) {
                            Ok(s) => {
                                this.str_buf.push_str(s);
                                this.byte_buf.clear();
                            }
                            Err(e) => {
                                return Poll::Ready(Some(Err(LlmError::StreamInterrupted(
                                    format!("Invalid UTF-8 in SSE stream: {e}"),
                                ))));
                            }
                        }
                    }
                    // Check if there's a final partial event in str_buf.
                    if this.str_buf.trim().is_empty() {
                        return Poll::Ready(None);
                    }
                    // Try to parse whatever remains
                    match try_parse_remaining(&mut this.str_buf) {
                        Ok(Some(chunk)) => return Poll::Ready(Some(Ok(chunk))),
                        Ok(None) => return Poll::Ready(None),
                        Err(e) => return Poll::Ready(Some(Err(e))),
                    }
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Convert complete UTF-8 sequences from `byte_buf` → `str_buf`.
/// Any trailing incomplete multi-byte sequence stays in `byte_buf`.
fn flush_utf8(byte_buf: &mut Vec<u8>, str_buf: &mut String) -> Result<(), LlmError> {
    if byte_buf.is_empty() {
        return Ok(());
    }
    match std::str::from_utf8(byte_buf) {
        // Entire buffer is valid UTF-8 — flush everything.
        Ok(s) => {
            str_buf.push_str(s);
            byte_buf.clear();
            Ok(())
        }
        // Partial UTF-8 — flush the valid prefix, keep the incomplete tail.
        Err(e) => {
            let valid_up_to = e.valid_up_to();
            if valid_up_to == 0 && e.error_len().is_some() {
                // No valid prefix at all, and it's a real error (not just incomplete).
                return Err(LlmError::StreamInterrupted(format!(
                    "Invalid UTF-8 in SSE stream: {e}"
                )));
            }
            // Flush the valid prefix.
            let valid = std::str::from_utf8(&byte_buf[..valid_up_to])
                .expect("valid_up_to is guaranteed to be a UTF-8 boundary");
            str_buf.push_str(valid);
            byte_buf.drain(..valid_up_to);
            // The remaining bytes are an incomplete multi-byte sequence —
            // they'll be completed when the next TCP chunk arrives.
            Ok(())
        }
    }
}

/// SSE event delimiter: double newline.
const SSE_EVENT_DELIMITER: &str = "\n\n";

/// SSE data line prefix.
const SSE_DATA_PREFIX: &str = "data:";

/// SSE stream termination sentinel value.
const SSE_DONE_MARKER: &str = "[DONE]";

/// Try to extract one complete SSE event from the buffer.
fn try_parse_event(buf: &mut String) -> Result<Option<ChatStreamChunk>, LlmError> {
    loop {
        let Some(boundary) = buf.find(SSE_EVENT_DELIMITER) else {
            return Ok(None);
        };

        let result = parse_sse_event(&buf[..boundary])?;
        buf.drain(..boundary + SSE_EVENT_DELIMITER.len());

        if let Some(chunk) = result {
            return Ok(Some(chunk));
        }
    }
}

/// Try to parse any remaining data in the buffer when the stream ends.
fn try_parse_remaining(buf: &mut String) -> Result<Option<ChatStreamChunk>, LlmError> {
    let text = std::mem::take(buf);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    parse_sse_event(trimmed)
}

/// Parse a single SSE event text block.
/// Returns `Ok(None)` for [DONE], comments, or empty data lines.
fn parse_sse_event(event_text: &str) -> Result<Option<ChatStreamChunk>, LlmError> {
    let mut data_parts = Vec::new();

    for line in event_text.lines() {
        if line.starts_with(':') {
            continue;
        }
        if let Some(rest) = line.strip_prefix(SSE_DATA_PREFIX) {
            let data = rest.strip_prefix(' ').unwrap_or(rest);
            data_parts.push(data);
        }
    }

    if data_parts.is_empty() {
        return Ok(None);
    }

    let data = data_parts.join("\n");
    let trimmed = data.trim();

    if trimmed == SSE_DONE_MARKER || trimmed.is_empty() {
        return Ok(None);
    }

    match serde_json::from_str::<ChatStreamChunk>(trimmed) {
        Ok(chunk) => Ok(Some(chunk)),
        Err(e) => Err(LlmError::Deserialize(format!(
            "Failed to parse SSE data: {e} | raw: {}",
            truncate_str(trimmed, 200)
        ))),
    }
}

fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        let end = (0..=max_len)
            .rev()
            .find(|&i| s.is_char_boundary(i))
            .unwrap_or(0);
        &s[..end]
    }
}
