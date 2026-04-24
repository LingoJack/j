use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::error::LlmError;
use super::types::ChatStreamChunk;

/// SSE stream: wraps a `reqwest::Response` byte stream, yields `ChatStreamChunk`.
pub struct SseStream {
    body: Pin<Box<dyn Stream<Item = Result<Vec<u8>, reqwest::Error>> + Send>>,
    buf: String,
}

impl SseStream {
    pub fn new(response: reqwest::Response) -> Self {
        use futures::StreamExt;
        Self {
            body: Box::pin(response.bytes_stream().map(|r| r.map(|b| b.to_vec()))),
            buf: String::new(),
        }
    }
}

impl Stream for SseStream {
    type Item = Result<ChatStreamChunk, LlmError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            // Try to extract a complete SSE event from the buffer
            if let Some(chunk) = try_parse_event(&mut this.buf)? {
                return Poll::Ready(Some(Ok(chunk)));
            }

            // Need more data from the body stream
            match Pin::new(&mut this.body).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => match std::str::from_utf8(&bytes) {
                    Ok(s) => this.buf.push_str(s),
                    Err(e) => {
                        return Poll::Ready(Some(Err(LlmError::StreamInterrupted(format!(
                            "Invalid UTF-8 in SSE stream: {e}"
                        )))));
                    }
                },
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(LlmError::StreamInterrupted(e.to_string()))));
                }
                Poll::Ready(None) => {
                    // Stream ended. Check if there's a final partial event in buf.
                    if this.buf.trim().is_empty() {
                        return Poll::Ready(None);
                    }
                    // Try to parse whatever remains
                    match try_parse_remaining(&mut this.buf) {
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

/// Try to extract one complete SSE event from the buffer.
fn try_parse_event(buf: &mut String) -> Result<Option<ChatStreamChunk>, LlmError> {
    loop {
        let Some(boundary) = buf.find("\n\n") else {
            return Ok(None);
        };

        let event_text = buf[..boundary].to_string();
        buf.drain(..boundary + 2);

        if let Some(chunk) = parse_sse_event(&event_text)? {
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
        if let Some(rest) = line.strip_prefix("data:") {
            let data = rest.strip_prefix(' ').unwrap_or(rest);
            data_parts.push(data);
        }
    }

    if data_parts.is_empty() {
        return Ok(None);
    }

    let data = data_parts.join("\n");
    let trimmed = data.trim();

    if trimmed == "[DONE]" || trimmed.is_empty() {
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
    if s.len() <= max_len { s } else { &s[..max_len] }
}
