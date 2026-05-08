//! oneshot 工具执行：工具调用处理 + 格式化时间

use crate::command::chat::app::types::{PlanDecision, ToolResultMsg};
use crate::command::chat::oneshot::confirm::interactive_confirm;
use crate::command::chat::oneshot::display::{print_tool_call_line, print_tool_result_line};
use crate::command::chat::permission::{JcliConfig, generate_allow_rule};
use crate::command::chat::storage::ToolCallItem;
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::classification::get_result_summary_for_tool;
use colored::Colorize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// 工具执行轮询间隔（毫秒）
const TOOL_POLL_MS: u64 = 30;

/// 处理单个工具调用：确认 → 执行 → 打印结果
///
/// 工具在子线程中执行，主线程轮询 `interrupted` 标志，检测到中断立即返回（放弃工具结果）。
/// 这样即使工具不响应 `cancelled` 标志（如 Read/Glob），用户按 Ctrl+C 也能立即退回 REPL。
pub(crate) fn handle_tool_call(
    item: &ToolCallItem,
    tool_registry: Arc<ToolRegistry>,
    jcli_config: &JcliConfig,
    cancelled: &Arc<AtomicBool>,
    interrupted: &Arc<AtomicBool>,
    bypass: bool,
) -> Option<ToolResultMsg> {
    // .jcli deny 检查
    if jcli_config.is_denied(&item.name, &item.arguments) {
        eprintln!(
            "  {} {} — {}",
            "✗".red(),
            item.name.red().bold(),
            "被权限规则拒绝".red()
        );
        return Some(ToolResultMsg {
            tool_call_id: item.id.clone(),
            result: "工具调用被拒绝（deny 规则匹配）".to_string(),
            is_error: true,
            images: vec![],
            plan_decision: PlanDecision::None,
        });
    }

    let needs_confirm = tool_registry
        .get(&item.name)
        .map(|t| t.requires_confirmation())
        .unwrap_or(false)
        && !jcli_config.is_allowed(&item.name, &item.arguments);

    if needs_confirm && !bypass {
        // 需要确认：先显示工具调用行
        print_tool_call_line(&item.name, &item.arguments);

        let allow_rule = generate_allow_rule(&item.name, &item.arguments);
        let options = ["允许执行", "拒绝", &format!("始终允许 ({})", allow_rule)];
        let choice = interactive_confirm(&item.name, &item.arguments, &options, 0);
        // 在确认期间用户可能按 Ctrl+C
        if interrupted.load(Ordering::Relaxed) {
            return None;
        }
        match choice {
            Some(0) => {}
            Some(2) => {
                // 始终允许
            }
            _ => {
                eprintln!(
                    "  {} {} — {}",
                    "⏭".dimmed(),
                    item.name.dimmed(),
                    "已跳过".dimmed()
                );
                return Some(ToolResultMsg {
                    tool_call_id: item.id.clone(),
                    result: "用户拒绝执行该工具".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                });
            }
        }
    } else {
        // 无需确认：直接显示工具调用行
        print_tool_call_line(&item.name, &item.arguments);
    }

    let start = std::time::Instant::now();

    // 工具在子线程中执行，避免阻塞主线程响应 Ctrl+C
    let (tx, rx) = std::sync::mpsc::channel();
    let tool_name = item.name.clone();
    let arguments = item.arguments.clone();
    let cancelled_thread = Arc::clone(cancelled);
    let registry_thread = Arc::clone(&tool_registry);
    std::thread::spawn(move || {
        let result = registry_thread.execute(&tool_name, &arguments, &cancelled_thread);
        let _ = tx.send(result);
    });

    // 轮询：每 TOOL_POLL_MS 检查一次中断标志和工具结果
    let result = loop {
        if interrupted.load(Ordering::Relaxed) {
            // 用户按了 Ctrl+C：放弃等待工具结果，让子线程后台跑完自己结束
            return None;
        }
        match rx.recv_timeout(Duration::from_millis(TOOL_POLL_MS)) {
            Ok(r) => break r,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // 子线程异常退出
                return Some(ToolResultMsg {
                    tool_call_id: item.id.clone(),
                    result: "[工具执行线程异常退出]".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                });
            }
        }
    };

    let elapsed = start.elapsed();
    let elapsed_str = format_duration(elapsed);

    let summary = get_result_summary_for_tool(
        &result.output,
        result.is_error,
        &item.name,
        Some(&item.arguments),
    );

    print_tool_result_line(&item.name, result.is_error, &summary, &elapsed_str);

    Some(ToolResultMsg {
        tool_call_id: item.id.clone(),
        result: result.output,
        is_error: result.is_error,
        images: vec![],
        plan_decision: PlanDecision::None,
    })
}

/// 格式化持续时间
pub(crate) fn format_duration(d: std::time::Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}
