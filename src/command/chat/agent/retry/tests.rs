use super::*;
use crate::command::chat::error::ChatError;

// ── 辅助：验证策略参数 ──

fn assert_policy(err: &ChatError, expected_max: u32, expected_base: u64, expected_cap: u64) {
    let policy = retry_policy_for(err)
        .unwrap_or_else(|| panic!("应为 {err:?} 返回 Some(RetryPolicy)，但得到 None"));
    assert_eq!(
        policy.max_attempts, expected_max,
        "{err:?}: max_attempts 不匹配"
    );
    assert_eq!(policy.base_ms, expected_base, "{err:?}: base_ms 不匹配");
    assert_eq!(policy.cap_ms, expected_cap, "{err:?}: cap_ms 不匹配");
}

fn assert_no_retry(err: &ChatError) {
    assert!(
        retry_policy_for(err).is_none(),
        "{err:?} 不应重试，但得到了策略"
    );
}

// ════════════════════════════════════════════════════════════════
// 回归测试：retry_policy_for 映射表
// 如果以下测试失败，说明错误→重试策略的映射被意外修改
// ════════════════════════════════════════════════════════════════

#[test]
fn retry_network_timeout_is_fast() {
    // NetworkTimeout → network_transient: 快速重试
    assert_policy(
        &ChatError::NetworkTimeout("conn timed out".into()),
        5,     // max_attempts (AGGRESSIVE)
        1000,  // base_ms (FAST)
        30000, // cap_ms (DEFAULT)
    );
}

#[test]
fn retry_stream_interrupted_is_fast() {
    // StreamInterrupted 与 NetworkTimeout 同策略
    assert_policy(
        &ChatError::StreamInterrupted("sse closed".into()),
        5,
        1000,
        30000,
    );
}

#[test]
fn retry_network_error_is_medium() {
    assert_policy(
        &ChatError::NetworkError("dns failure".into()),
        5,     // AGGRESSIVE
        2000,  // MEDIUM
        30000, // DEFAULT
    );
}

#[test]
fn retry_stream_deserialize() {
    assert_policy(
        &ChatError::StreamDeserialize("invalid json".into()),
        3,     // CONSERVATIVE
        2000,  // MEDIUM
        20000, // ABNORMAL
    );
}

#[test]
fn retry_503_is_overloaded() {
    assert_policy(
        &ChatError::ApiServerError {
            status: 503,
            message: "unavailable".into(),
        },
        4,     // MODERATE
        2000,  // MEDIUM
        30000, // DEFAULT
    );
}

#[test]
fn retry_504_is_overloaded() {
    assert_policy(
        &ChatError::ApiServerError {
            status: 504,
            message: "gateway timeout".into(),
        },
        4,
        2000,
        30000,
    );
}

#[test]
fn retry_529_is_overloaded() {
    assert_policy(
        &ChatError::ApiServerError {
            status: 529,
            message: "overloaded".into(),
        },
        4,
        2000,
        30000,
    );
}

#[test]
fn retry_500_is_server_error() {
    assert_policy(
        &ChatError::ApiServerError {
            status: 500,
            message: "internal error".into(),
        },
        3,     // CONSERVATIVE
        3000,  // SLOW
        30000, // DEFAULT
    );
}

#[test]
fn retry_502_is_server_error() {
    assert_policy(
        &ChatError::ApiServerError {
            status: 502,
            message: "bad gateway".into(),
        },
        3,
        3000,
        30000,
    );
}

#[test]
fn retry_429_with_retry_after() {
    // 429 + retry_after=30s → base=30000ms, max=1
    assert_policy(
        &ChatError::ApiRateLimit {
            message: "slow down".into(),
            retry_after_secs: Some(30),
        },
        1,      // ONCE
        30000,  // 30 * 1000
        120000, // RETRY_AFTER
    );
}

#[test]
fn retry_429_without_retry_after() {
    assert_policy(
        &ChatError::ApiRateLimit {
            message: "slow down".into(),
            retry_after_secs: None,
        },
        3,     // CONSERVATIVE
        5000,  // RATE_LIMIT
        60000, // RATE_LIMIT cap
    );
}

#[test]
fn retry_429_retry_after_capped_at_120s() {
    // retry_after=200s → 被截断到 120s
    assert_policy(
        &ChatError::ApiRateLimit {
            message: "slow down".into(),
            retry_after_secs: Some(200),
        },
        1,
        120000, // min(200, 120) * 1000 = 120000
        120000,
    );
}

#[test]
fn retry_abnormal_finish_network() {
    assert_policy(
        &ChatError::AbnormalFinish("network_error".into()),
        3,
        2000,
        20000,
    );
}

#[test]
fn retry_abnormal_finish_timeout() {
    assert_policy(&ChatError::AbnormalFinish("timeout".into()), 3, 2000, 20000);
}

#[test]
fn retry_abnormal_finish_overloaded() {
    assert_policy(
        &ChatError::AbnormalFinish("overloaded".into()),
        3,
        2000,
        20000,
    );
}

#[test]
fn retry_abnormal_finish_other_no_retry() {
    assert_no_retry(&ChatError::AbnormalFinish("content_filter".into()));
    assert_no_retry(&ChatError::AbnormalFinish("max_tokens".into()));
    assert_no_retry(&ChatError::AbnormalFinish("stop".into()));
}

#[test]
fn retry_other_overloaded_keywords() {
    // 兜底：Other 中包含过载关键词 → fallback_overloaded
    let keywords = [
        "访问量过大，请稍后",
        "服务过载",
        "server overloaded now",
        "too busy to handle",
        "Error code: 1305",
    ];
    for kw in keywords {
        assert_policy(
            &ChatError::Other(kw.into()),
            3,     // CONSERVATIVE
            3000,  // SLOW
            30000, // DEFAULT
        );
    }
}

#[test]
fn retry_non_retryable_errors() {
    // 以下错误类型不应触发重试
    assert_no_retry(&ChatError::ApiAuth("bad key".into()));
    assert_no_retry(&ChatError::ApiBadRequest("invalid param".into()));
    assert_no_retry(&ChatError::HookAborted);
    assert_no_retry(&ChatError::RuntimeFailed("something broke".into()));
    assert_no_retry(&ChatError::AgentPanic("thread panicked".into()));
    assert_no_retry(&ChatError::RequestBuild("bad args".into()));
    assert_no_retry(&ChatError::Other("unknown error".into()));
}

// ════════════════════════════════════════════════════════════════
// 回归测试：backoff_delay_ms 计算
// ════════════════════════════════════════════════════════════════

#[test]
fn backoff_first_attempt_in_range() {
    // attempt=1, base=1000, cap=30000
    // exp = 1000, jitter ∈ [0, 200], 结果 ∈ [1000, 1200]
    for _ in 0..100 {
        let delay = backoff_delay_ms(1, 1000, 30000);
        assert!(
            (1000..=1200).contains(&delay),
            "attempt=1 delay={delay} 不在 [1000, 1200] 范围内"
        );
    }
}

#[test]
fn backoff_exponential_growth() {
    // attempt=2, base=1000: exp=2000, jitter ∈ [0, 400], 结果 ∈ [2000, 2400]
    // attempt=3, base=1000: exp=4000, jitter ∈ [0, 800], 结果 ∈ [4000, 4800]
    for _ in 0..50 {
        let d2 = backoff_delay_ms(2, 1000, 30000);
        assert!(
            (2000..=2400).contains(&d2),
            "attempt=2 delay={d2} 不在 [2000, 2400] 范围内"
        );
        let d3 = backoff_delay_ms(3, 1000, 30000);
        assert!(
            (4000..=4800).contains(&d3),
            "attempt=3 delay={d3} 不在 [4000, 4800] 范围内"
        );
    }
}

#[test]
fn backoff_capped_at_cap() {
    // attempt=10, base=1000, cap=30000
    // 理论 exp = 1000 * 2^9 = 512000, 但被 cap 限制为 30000
    // jitter ∈ [0, 6000], 结果 ∈ [30000, 36000]
    for _ in 0..50 {
        let delay = backoff_delay_ms(10, 1000, 30000);
        assert!(
            (30000..=36000).contains(&delay),
            "attempt=10 delay={delay} 不在 [30000, 36000] 范围内（未正确 cap）"
        );
    }
}

#[test]
fn backoff_never_below_base() {
    // 即使 base=1, delay 至少为 1 (exp=1, jitter=0)
    for _ in 0..50 {
        let delay = backoff_delay_ms(1, 1, 1000);
        assert!(delay >= 1, "delay={delay} < 1，退避延迟不应小于 base");
    }
}

#[test]
fn backoff_monotonically_non_decreasing() {
    // 验证退避延迟的指数基数部分随 attempt 单调非递减
    // 由于 jitter 的随机性，实际值可能波动，但指数基数 exp = base * 2^(attempt-1) 必然非递减
    let base = 2000u64;
    let cap = 30000u64;
    let mut prev_exp = 0u64;
    for attempt in 1..=8 {
        // 计算 exp = base * 2^(attempt-1)，cap 限制
        let shift = (attempt - 1).min(6) as u64; // BACKOFF_MAX_SHIFT
        let exp = base.saturating_mul(1u64 << shift).min(cap);
        assert!(
            exp >= prev_exp,
            "attempt={attempt} exp={exp} < prev_exp={prev_exp}，指数基数应单调非递减"
        );
        prev_exp = exp;
    }
    // 额外验证：attempt=1 和 attempt=10 的实际 delay 采样
    let d1_min = backoff_delay_ms(1, base, cap);
    let d10_max = backoff_delay_ms(10, base, cap);
    assert!(
        d10_max >= d1_min,
        "attempt=10 的 delay ({d10_max}) 应 >= attempt=1 的 delay ({d1_min})"
    );
}
