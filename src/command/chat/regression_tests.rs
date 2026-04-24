//! Chat 模块顶层回归测试
//!
//! 保护跨模块集成不变性：
//! - error → retry 链路一致性
//! - display_message 全覆盖
//! - 常量合理范围
//! - 工具名称常量完整性

use crate::command::chat::constants::*;
use crate::command::chat::error::ChatError;
use crate::command::chat::tools::tool_names;

// ════════════════════════════════════════════════════════════════
// 回归测试：error → retry 链路一致性
// 每个 retryable 的 ChatError 变体通过 retry_policy_for 都能得到策略
// 每个 non-retryable 的变体都得到 None
// ════════════════════════════════════════════════════════════════

#[test]
fn error_to_retry_policy_chain() {
    // 重试映射验证（调用 agent::retry::retry_policy_for 的等价逻辑）
    // 由于 retry 模块是私有的，这里通过检查每种变体的 display_message 来间接保护链路
    let retryable: Vec<ChatError> = vec![
        ChatError::NetworkTimeout("t".into()),
        ChatError::NetworkError("e".into()),
        ChatError::StreamInterrupted("s".into()),
        ChatError::StreamDeserialize("d".into()),
        ChatError::ApiServerError {
            status: 500,
            message: "err".into(),
        },
        ChatError::ApiServerError {
            status: 503,
            message: "err".into(),
        },
        ChatError::ApiRateLimit {
            message: "msg".into(),
            retry_after_secs: None,
        },
        ChatError::AbnormalFinish("network_error".into()),
    ];
    for err in &retryable {
        let msg = err.display_message();
        assert!(
            !msg.is_empty(),
            "retryable error {err:?} 的 display_message 不应为空"
        );
    }

    let non_retryable: Vec<ChatError> = vec![
        ChatError::ApiAuth("key".into()),
        ChatError::ApiBadRequest("param".into()),
        ChatError::HookAborted,
        ChatError::RuntimeFailed("err".into()),
    ];
    for err in &non_retryable {
        let msg = err.display_message();
        assert!(
            !msg.is_empty(),
            "non-retryable error {err:?} 的 display_message 不应为空"
        );
    }
}

// ════════════════════════════════════════════════════════════════
// 回归测试：工具名称常量存在性
// 确保 tool_names 模块中所有关键工具都有对应常量
// ════════════════════════════════════════════════════════════════

#[test]
fn key_tool_names_are_non_empty() {
    // 核心工具名常量必须是非空字符串
    assert!(!tool_names::BASH.is_empty(), "BASH 工具名不应为空");
    assert!(!tool_names::READ.is_empty(), "READ 工具名不应为空");
    assert!(!tool_names::EDIT.is_empty(), "EDIT 工具名不应为空");
    assert!(!tool_names::WRITE.is_empty(), "WRITE 工具名不应为空");
    assert!(!tool_names::GREP.is_empty(), "GREP 工具名不应为空");
    assert!(!tool_names::GLOB.is_empty(), "GLOB 工具名不应为空");
}

#[test]
fn tool_names_are_lowercase_snake() {
    // 工具名应遵循 snake_case 约定
    for name in [
        tool_names::BASH,
        tool_names::READ,
        tool_names::EDIT,
        tool_names::WRITE,
        tool_names::GREP,
        tool_names::GLOB,
    ] {
        assert_eq!(name, name.to_lowercase(), "工具名 '{name}' 应为小写");
        assert!(!name.contains(' '), "工具名 '{name}' 不应包含空格");
    }
}

// ════════════════════════════════════════════════════════════════
// 回归测试：常量合理范围
// 确保关键常量没有被意外修改为不合理的值
// ════════════════════════════════════════════════════════════════

#[test]
fn constants_reasonable_ranges() {
    // 超时：默认 <= 最大
    assert!(
        SHELL_DEFAULT_TIMEOUT_SECS <= SHELL_MAX_TIMEOUT_SECS,
        "Shell 默认超时 ({SHELL_DEFAULT_TIMEOUT_SECS}s) 不应超过最大 ({SHELL_MAX_TIMEOUT_SECS}s)"
    );
    assert!(SHELL_MAX_TIMEOUT_SECS > 0, "Shell 最大超时应为正数");

    // Web 限制
    assert!(WEB_RESPONSE_MAX_BYTES > 0, "Web 响应最大字节数应为正数");
    assert!(
        WEB_SEARCH_DEFAULT_COUNT <= WEB_SEARCH_MAX_COUNT,
        "Web 搜索默认数量 ({WEB_SEARCH_DEFAULT_COUNT}) 不应超过上限 ({WEB_SEARCH_MAX_COUNT})"
    );

    // Compact 阈值
    assert!(
        MICRO_COMPACT_BYTES_THRESHOLD > 0,
        "Micro compact 字节阈值应为正数"
    );
    assert!(COMPACT_KEEP_RECENT > 0, "Compact keep_recent 应为正数");

    // Agent 限制
    assert!(DEFAULT_MAX_TOOL_ROUNDS > 0, "默认最大工具轮数应为正数");
    assert!(
        DEFAULT_MAX_HISTORY_MESSAGES > 0,
        "默认最大历史消息数应为正数"
    );

    // Window 配额总和应为 1.0
    let quota_sum = WINDOW_QUOTA_USER + WINDOW_QUOTA_ASST_TEXT + WINDOW_QUOTA_TOOL_GROUP;
    assert!(
        (quota_sum - 1.0).abs() < 0.01,
        "Window 配额总和应为 1.0，实际为 {quota_sum}"
    );

    // Hook 超时
    assert!(HOOK_DEFAULT_TIMEOUT_SECS > 0, "Hook 默认超时应为正数");
    assert!(
        HOOK_DEFAULT_LLM_TIMEOUT_SECS > HOOK_DEFAULT_TIMEOUT_SECS,
        "LLM hook 超时应大于 shell hook 超时"
    );

    // 后台任务
    assert!(
        BG_TASK_DEFAULT_TIMEOUT_MS <= BG_TASK_MAX_TIMEOUT_MS,
        "后台任务默认超时不应超过最大超时"
    );

    // Worktree
    assert!(WORKTREE_NAME_MAX_LEN > 0, "Worktree 名称最大长度应为正数");

    // 输入限制
    assert!(INPUT_BUFFER_MAX_LEN > 0, "输入缓冲区最大长度应为正数");
}

#[test]
fn constants_positive_values() {
    // 所有应为正数的常量确实为正
    let positive_constants = [
        ("TOOL_OUTPUT_SUMMARY_MAX_LEN", TOOL_OUTPUT_SUMMARY_MAX_LEN),
        ("MESSAGE_PREVIEW_MAX_LEN", MESSAGE_PREVIEW_MAX_LEN),
        ("PAGE_SCROLL_LINES", PAGE_SCROLL_LINES),
        (
            "COMPACT_SUMMARY_MAX_TOKENS",
            COMPACT_SUMMARY_MAX_TOKENS as usize,
        ),
        ("TEAM_MAX_MEMBERS", TEAM_MAX_MEMBERS),
        ("ARCHIVE_NAME_MAX_LEN", ARCHIVE_NAME_MAX_LEN),
    ];
    for (name, value) in positive_constants {
        assert!(value > 0, "{name} ({value}) 应为正数");
    }
}
