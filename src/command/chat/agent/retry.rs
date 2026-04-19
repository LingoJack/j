use super::super::error::ChatError;
use rand::Rng;

/// 每种可重试错误的重试策略
pub(super) struct RetryPolicy {
    /// 最大重试次数（不含首次请求）
    pub(super) max_attempts: u32,
    /// 首次退避基础延迟（毫秒）
    pub(super) base_ms: u64,
    /// 延迟上限（毫秒）
    pub(super) cap_ms: u64,
}

/// 根据错误类型确定重试策略
///
/// 策略设计原则：
/// - 网络瞬断（超时/断连）：快速重试，基础 1s，最多 5 次
/// - 5xx 服务端过载（503/504/529）：稍慢重试，基础 2s，最多 4 次
/// - 5xx 服务端错误（500/502）：再慢一些，基础 3s，最多 3 次
/// - 429 有 retry_after：精确等待（上限 120s），只重试 1 次
/// - 429 无 retry_after：保守重试，基础 5s，最多 3 次
/// - 非标准 finish_reason（如 network_error）：中等重试
pub(super) fn retry_policy_for(error: &ChatError) -> Option<RetryPolicy> {
    match error {
        ChatError::NetworkTimeout(_) | ChatError::StreamInterrupted(_) => Some(RetryPolicy {
            max_attempts: 5,
            base_ms: 1_000,
            cap_ms: 30_000,
        }),
        ChatError::NetworkError(_) => Some(RetryPolicy {
            max_attempts: 5,
            base_ms: 2_000,
            cap_ms: 30_000,
        }),
        ChatError::ApiServerError { status, .. } => match status {
            503 | 504 | 529 => Some(RetryPolicy {
                max_attempts: 4,
                base_ms: 2_000,
                cap_ms: 30_000,
            }),
            500 | 502 => Some(RetryPolicy {
                max_attempts: 3,
                base_ms: 3_000,
                cap_ms: 30_000,
            }),
            _ => None,
        },
        ChatError::ApiRateLimit {
            retry_after_secs: Some(secs),
            ..
        } => {
            // 有明确的等待时间：等待指定时长（上限 120s），只重试一次
            let wait = (*secs).min(120);
            Some(RetryPolicy {
                max_attempts: 1,
                base_ms: wait * 1_000,
                cap_ms: 120_000,
            })
        }
        ChatError::ApiRateLimit {
            retry_after_secs: None,
            ..
        } => Some(RetryPolicy {
            max_attempts: 3,
            base_ms: 5_000,
            cap_ms: 60_000,
        }),
        ChatError::AbnormalFinish(reason)
            if matches!(reason.as_str(), "network_error" | "timeout" | "overloaded") =>
        {
            Some(RetryPolicy {
                max_attempts: 3,
                base_ms: 2_000,
                cap_ms: 20_000,
            })
        }
        // 兜底：Other 中包含过载/访问量过大关键词（部分 API 错误未被正确分类时）
        ChatError::Other(msg)
            if msg.contains("访问量过大")
                || msg.contains("过载")
                || msg.contains("overloaded")
                || msg.contains("too busy")
                || msg.contains("1305") =>
        {
            Some(RetryPolicy {
                max_attempts: 3,
                base_ms: 3_000,
                cap_ms: 30_000,
            })
        }
        _ => None,
    }
}

/// 计算第 `attempt`（从 1 开始）次重试的退避延迟（毫秒）
///
/// 公式：`clamp(base * 2^(attempt-1), 0, cap) + jitter(0..20%)`
pub(super) fn backoff_delay_ms(attempt: u32, base_ms: u64, cap_ms: u64) -> u64 {
    // 最多移位 10 次，避免溢出
    let shift = (attempt - 1).min(10) as u64;
    let exp = base_ms.saturating_mul(1u64 << shift).min(cap_ms);
    // 加 0–20% 随机抖动，分散并发重试
    let jitter = rand::thread_rng().gen_range(0..=(exp / 5));
    exp + jitter
}
