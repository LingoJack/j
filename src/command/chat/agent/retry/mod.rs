use super::super::error::ChatError;
use rand::Rng;

// ── 退避延迟常量 ──────────────────────────────────────────────

/// 退避指数移位上限，防止溢出
const BACKOFF_MAX_SHIFT: u64 = 10;
/// 抖动分母：抖动范围 = exp / JITTER_DIVISOR，即 0–20%
const JITTER_DIVISOR: u64 = 5;

// ── 基础延迟常量（毫秒） ──────────────────────────────────────

const BASE_FAST_MS: u64 = 1_000;
const BASE_MEDIUM_MS: u64 = 2_000;
const BASE_SLOW_MS: u64 = 3_000;
const BASE_RATE_LIMIT_MS: u64 = 5_000;

// ── 延迟上限常量（毫秒） ──────────────────────────────────────

const CAP_DEFAULT_MS: u64 = 30_000;
const CAP_ABNORMAL_MS: u64 = 20_000;
const CAP_RATE_LIMIT_MS: u64 = 60_000;
const CAP_RETRY_AFTER_MS: u64 = 120_000;

// ── 最大重试次数 ──────────────────────────────────────────────

const MAX_ATTEMPTS_AGGRESSIVE: u32 = 5;
const MAX_ATTEMPTS_MODERATE: u32 = 4;
const MAX_ATTEMPTS_CONSERVATIVE: u32 = 3;
const MAX_ATTEMPTS_ONCE: u32 = 1;

// ── 429 retry_after 上限（秒） ────────────────────────────────

const RETRY_AFTER_CAP_SECS: u64 = 120;

/// 每种可重试错误的重试策略
#[derive(Debug)]
pub(super) struct RetryPolicy {
    /// 最大重试次数（不含首次请求）
    pub(super) max_attempts: u32,
    /// 首次退避基础延迟（毫秒）
    pub(super) base_ms: u64,
    /// 延迟上限（毫秒）
    pub(super) cap_ms: u64,
}

impl RetryPolicy {
    /// 网络瞬断策略：快速重试，基础 1s，最多 5 次
    const fn network_transient() -> Self {
        Self {
            max_attempts: MAX_ATTEMPTS_AGGRESSIVE,
            base_ms: BASE_FAST_MS,
            cap_ms: CAP_DEFAULT_MS,
        }
    }

    /// 网络错误策略：基础 2s，最多 5 次
    const fn network_error() -> Self {
        Self {
            max_attempts: MAX_ATTEMPTS_AGGRESSIVE,
            base_ms: BASE_MEDIUM_MS,
            cap_ms: CAP_DEFAULT_MS,
        }
    }

    /// 服务端过载策略（503/504/529）：基础 2s，最多 4 次
    const fn server_overloaded() -> Self {
        Self {
            max_attempts: MAX_ATTEMPTS_MODERATE,
            base_ms: BASE_MEDIUM_MS,
            cap_ms: CAP_DEFAULT_MS,
        }
    }

    /// 服务端错误策略（500/502）：基础 3s，最多 3 次
    const fn server_error() -> Self {
        Self {
            max_attempts: MAX_ATTEMPTS_CONSERVATIVE,
            base_ms: BASE_SLOW_MS,
            cap_ms: CAP_DEFAULT_MS,
        }
    }

    /// 429 有 retry_after：精确等待（上限 120s），只重试 1 次
    fn rate_limit_with_retry_after(secs: u64) -> Self {
        let wait_ms = secs.min(RETRY_AFTER_CAP_SECS) * 1_000;
        Self {
            max_attempts: MAX_ATTEMPTS_ONCE,
            base_ms: wait_ms,
            cap_ms: CAP_RETRY_AFTER_MS,
        }
    }

    /// 429 无 retry_after：保守重试，基础 5s，最多 3 次
    const fn rate_limit_blind() -> Self {
        Self {
            max_attempts: MAX_ATTEMPTS_CONSERVATIVE,
            base_ms: BASE_RATE_LIMIT_MS,
            cap_ms: CAP_RATE_LIMIT_MS,
        }
    }

    /// 非标准 finish_reason（如 network_error）：中等重试
    const fn abnormal_finish() -> Self {
        Self {
            max_attempts: MAX_ATTEMPTS_CONSERVATIVE,
            base_ms: BASE_MEDIUM_MS,
            cap_ms: CAP_ABNORMAL_MS,
        }
    }

    /// 响应解析失败策略：服务端返回格式异常/SSE 截断/HTML 错误页等，通常为瞬时问题
    const fn deserialize_error() -> Self {
        Self {
            max_attempts: MAX_ATTEMPTS_CONSERVATIVE,
            base_ms: BASE_MEDIUM_MS,
            cap_ms: CAP_ABNORMAL_MS,
        }
    }

    /// 兜底过载策略：Other 中包含过载/访问量过大关键词
    const fn fallback_overloaded() -> Self {
        Self {
            max_attempts: MAX_ATTEMPTS_CONSERVATIVE,
            base_ms: BASE_SLOW_MS,
            cap_ms: CAP_DEFAULT_MS,
        }
    }
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
/// - 响应解析失败（JSON 格式异常/SSE 截断）：中等重试，fallback 非流式失败后的兜底
pub(super) fn retry_policy_for(error: &ChatError) -> Option<RetryPolicy> {
    match error {
        ChatError::NetworkTimeout(_) | ChatError::StreamInterrupted(_) => {
            Some(RetryPolicy::network_transient())
        }
        ChatError::NetworkError(_) => Some(RetryPolicy::network_error()),
        ChatError::StreamDeserialize(_) => Some(RetryPolicy::deserialize_error()),
        ChatError::ApiServerError { status, .. } => match status {
            503 | 504 | 529 => Some(RetryPolicy::server_overloaded()),
            500 | 502 => Some(RetryPolicy::server_error()),
            _ => None,
        },
        ChatError::ApiRateLimit {
            retry_after_secs: Some(secs),
            ..
        } => Some(RetryPolicy::rate_limit_with_retry_after(*secs)),
        ChatError::ApiRateLimit {
            retry_after_secs: None,
            ..
        } => Some(RetryPolicy::rate_limit_blind()),
        ChatError::AbnormalFinish(reason)
            if matches!(reason.as_str(), "network_error" | "timeout" | "overloaded") =>
        {
            Some(RetryPolicy::abnormal_finish())
        }
        // 兜底：Other 中包含过载/访问量过大关键词（部分 API 错误未被正确分类时）
        ChatError::Other(msg)
            if msg.contains("访问量过大")
                || msg.contains("过载")
                || msg.contains("overloaded")
                || msg.contains("too busy")
                || msg.contains("1305") =>
        {
            Some(RetryPolicy::fallback_overloaded())
        }
        _ => None,
    }
}

/// 计算第 `attempt`（从 1 开始）次重试的退避延迟（毫秒）
///
/// 公式：`clamp(base * 2^(attempt-1), 0, cap) + jitter(0..20%)`
pub(super) fn backoff_delay_ms(attempt: u32, base_ms: u64, cap_ms: u64) -> u64 {
    let shift = (attempt - 1).min(BACKOFF_MAX_SHIFT as u32) as u64;
    let exp = base_ms.saturating_mul(1u64 << shift).min(cap_ms);
    // 加 0–20% 随机抖动，分散并发重试
    let jitter = rand::thread_rng().gen_range(0..=(exp / JITTER_DIVISOR));
    exp + jitter
}

#[cfg(test)]
mod tests;
