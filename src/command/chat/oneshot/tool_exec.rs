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
use std::sync::atomic::AtomicBool;

/// 处理单个工具调用：确认 → 执行 → 打印结果
pub(crate) fn handle_tool_call(
    item: &ToolCallItem,
    tool_registry: &ToolRegistry,
    jcli_config: &JcliConfig,
    cancelled: &Arc<AtomicBool>,
    bypass: bool,
) -> ToolResultMsg {
    // .jcli deny 检查
    if jcli_config.is_denied(&item.name, &item.arguments) {
        eprintln!(
            "  {} {} — {}",
            "✗".red(),
            item.name.red().bold(),
            "被权限规则拒绝".red()
        );
        return ToolResultMsg {
            tool_call_id: item.id.clone(),
            result: "工具调用被拒绝（deny 规则匹配）".to_string(),
            is_error: true,
            images: vec![],
            plan_decision: PlanDecision::None,
        };
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
                return ToolResultMsg {
                    tool_call_id: item.id.clone(),
                    result: "用户拒绝执行该工具".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        }
    } else {
        // 无需确认：直接显示工具调用行
        print_tool_call_line(&item.name, &item.arguments);
    }

    let start = std::time::Instant::now();
    let result = tool_registry.execute(&item.name, &item.arguments, cancelled);
    let elapsed = start.elapsed();
    let elapsed_str = format_duration(elapsed);

    let summary = get_result_summary_for_tool(
        &result.output,
        result.is_error,
        &item.name,
        Some(&item.arguments),
    );

    print_tool_result_line(&item.name, result.is_error, &summary, &elapsed_str);

    ToolResultMsg {
        tool_call_id: item.id.clone(),
        result: result.output,
        is_error: result.is_error,
        images: vec![],
        plan_decision: PlanDecision::None,
    }
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
