use super::definition::*;
use super::executor::{
    execute_hook_with_provider, execute_llm_hook, execute_shell_hook, extract_json_from_llm_output,
    render_prompt_template,
};
use super::manager::*;
use super::types::*;
use std::sync::Arc;

#[test]
fn test_hook_event_roundtrip() {
    for event in HookEvent::all() {
        let s = event.as_str();
        let parsed = HookEvent::parse(s).unwrap();
        assert_eq!(*event, parsed);
    }
}

#[test]
fn test_hook_event_from_str_invalid() {
    assert!(HookEvent::parse("unknown_event").is_none());
}

#[test]
fn test_hook_def_default_timeout() {
    let yaml = r#"command: "echo hello""#;
    let def: HookDef = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(def.timeout, 10);
    assert_eq!(def.r#type, HookType::Bash);
}

#[test]
fn test_hook_def_to_hook_kind_bash() {
    let def = HookDef {
        r#type: HookType::Bash,
        command: Some("echo test".to_string()),
        prompt: None,
        model: None,
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
    };
    let kind = HookKind::from(def);
    match kind {
        HookKind::Shell(shell) => {
            assert_eq!(shell.command, "echo test");
            assert_eq!(shell.timeout, 5);
        }
        _ => panic!("应该转换为 Shell 变体"),
    }
}

#[test]
fn test_hook_def_to_hook_kind_llm() {
    let def = HookDef {
        r#type: HookType::Llm,
        command: None,
        prompt: Some("检查敏感信息: {{user_input}}".to_string()),
        model: Some("gpt-4o".to_string()),
        timeout: 10, // 使用默认 timeout → 应被替换为 30
        retry: 2,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
    };
    let kind = def.into_hook_kind().unwrap();
    match kind {
        HookKind::Llm(llm) => {
            assert_eq!(llm.prompt, "检查敏感信息: {{user_input}}");
            assert_eq!(llm.model.as_deref(), Some("gpt-4o"));
            assert_eq!(llm.timeout, 30); // 默认 timeout 被替换为 llm 默认值
            assert_eq!(llm.retry, 2);
        }
        _ => panic!("应该转换为 Llm 变体"),
    }
}

#[test]
fn test_hook_def_llm_explicit_timeout() {
    let def = HookDef {
        r#type: HookType::Llm,
        command: None,
        prompt: Some("test prompt".to_string()),
        model: None,
        timeout: 60,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
    };
    let kind = def.into_hook_kind().unwrap();
    match kind {
        HookKind::Llm(llm) => {
            assert_eq!(llm.timeout, 60); // 显式设置的超时保留
        }
        _ => panic!("应该转换为 Llm 变体"),
    }
}

#[test]
fn test_hook_def_yaml_with_type() {
    let yaml = r#"
type: llm
prompt: "检查敏感信息"
model: gpt-4o
timeout: 30
retry: 2
"#;
    let def: HookDef = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(def.r#type, HookType::Llm);
    assert_eq!(def.prompt.as_deref(), Some("检查敏感信息"));
    assert_eq!(def.model.as_deref(), Some("gpt-4o"));
    assert_eq!(def.timeout, 30);
    assert_eq!(def.retry, 2);
}

#[test]
fn test_hook_result_empty_json() {
    let result: HookResult = serde_json::from_str("{}").unwrap();
    assert!(!result.is_halt());
    assert!(result.messages.is_none());
    assert!(result.user_input.is_none());
}

#[test]
fn test_hook_result_with_stop() {
    // action=stop 中止当前步骤
    let json = r#"{"action": "stop"}"#;
    let result: HookResult = serde_json::from_str(json).unwrap();
    assert!(result.is_stop());
}

#[test]
fn test_hook_result_with_action_stop() {
    let json = r#"{"action": "stop"}"#;
    let result: HookResult = serde_json::from_str(json).unwrap();
    assert!(result.is_stop());
    assert!(!result.is_skip());
}

#[test]
fn test_hook_result_with_action_skip() {
    let json = r#"{"action": "skip"}"#;
    let result: HookResult = serde_json::from_str(json).unwrap();
    assert!(result.is_skip());
    assert!(!result.is_stop());
}

#[test]
fn test_hook_result_with_user_input() {
    let json = r#"{"user_input": "[modified] hello"}"#;
    let result: HookResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.user_input.as_deref(), Some("[modified] hello"));
}

#[test]
fn test_hook_context_serialization() {
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("hello".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_string(&ctx).unwrap();
    assert!(json.contains("pre_send_message"));
    assert!(json.contains("hello"));
    assert!(json.contains("user_input"));
    // skip_serializing_if 应跳过 None 字段
    assert!(!json.contains("messages"));
    assert!(!json.contains("tool_name"));
}

#[test]
fn test_execute_shell_hook_echo() {
    let hook = ShellHook {
        name: None,
        command: r#"echo '{"user_input": "hooked"}'"#.to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("original".to_string()),
        ..Default::default()
    };
    let result = execute_shell_hook(&hook, &ctx).unwrap();
    assert_eq!(result.user_input.as_deref(), Some("hooked"));
    assert!(!result.is_halt());
}

#[test]
fn test_execute_shell_hook_empty_output() {
    let hook = ShellHook {
        name: None,
        command: "echo ''".to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext::default();
    let result = execute_shell_hook(&hook, &ctx).unwrap();
    assert!(!result.is_halt());
    assert!(result.user_input.is_none());
}

#[test]
fn test_execute_shell_hook_nonzero_exit() {
    let hook = ShellHook {
        name: None,
        command: "exit 1".to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext::default();
    let result = execute_shell_hook(&hook, &ctx);
    assert!(result.is_err());
}

#[test]
fn test_execute_shell_hook_reads_stdin() {
    let hook = ShellHook {
        name: None,
        command: r#"input=$(cat); event=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('event',''))" 2>/dev/null || echo ""); echo '{"user_input": "got_input"}'"#.to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("test".to_string()),
        ..Default::default()
    };
    let result = execute_shell_hook(&hook, &ctx).unwrap();
    assert_eq!(result.user_input.as_deref(), Some("got_input"));
}

#[test]
fn test_execute_builtin_hook() {
    let builtin = BuiltinHook {
        name: "test_hook".to_string(),
        handler: Arc::new(|ctx| {
            ctx.user_input.as_ref().map(|input| HookResult {
                user_input: Some(format!("[hooked] {}", input)),
                ..Default::default()
            })
        }),
    };
    let kind = HookKind::Builtin(builtin);
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("original".to_string()),
        ..Default::default()
    };
    let result = execute_hook_with_provider(&kind, &ctx, &None).unwrap();
    assert_eq!(result.user_input.as_deref(), Some("[hooked] original"));
}

#[test]
fn test_execute_builtin_hook_returns_none() {
    let builtin = BuiltinHook {
        name: "no_op".to_string(),
        handler: Arc::new(|_| None),
    };
    let kind = HookKind::Builtin(builtin);
    let ctx = HookContext::default();
    let result = execute_hook_with_provider(&kind, &ctx, &None).unwrap();
    assert!(!result.is_halt());
    assert!(result.user_input.is_none());
}

#[test]
fn test_hook_manager_empty() {
    let manager = HookManager::default();
    assert!(manager.list_hooks().is_empty());
    let result = manager.execute(HookEvent::PreSendMessage, HookContext::default(), &[]);
    assert!(result.is_none());
}

#[test]
fn test_hook_manager_session_hooks() {
    let mut manager = HookManager::default();
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "session_hooked"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let hooks = manager.list_hooks();
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].source, "session");

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                user_input: Some("original".to_string()),
                ..Default::default()
            },
            &[],
        )
        .unwrap();
    assert_eq!(result.user_input.as_deref(), Some("session_hooked"));
}

#[test]
fn test_hook_manager_builtin_hooks() {
    let mut manager = HookManager::default();
    manager.register_builtin(HookEvent::PreSendMessage, "test_builtin", |ctx| {
        ctx.user_input.as_ref().map(|input| HookResult {
            user_input: Some(format!("[builtin] {}", input)),
            ..Default::default()
        })
    });

    let hooks = manager.list_hooks();
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].source, "builtin");
    assert!(hooks[0].label.contains("test_builtin"));

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                user_input: Some("hello".to_string()),
                ..Default::default()
            },
            &[],
        )
        .unwrap();
    assert_eq!(result.user_input.as_deref(), Some("[builtin] hello"));
}

#[test]
fn test_hook_manager_builtin_before_session() {
    // 内置 hook 应在 session hook 之前执行，session hook 应能覆盖内置 hook 的结果
    let mut manager = HookManager::default();
    manager.register_builtin(HookEvent::PreSendMessage, "prefix", |ctx| {
        ctx.user_input.as_ref().map(|input| HookResult {
            user_input: Some(format!("[builtin] {}", input)),
            ..Default::default()
        })
    });
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "session_overridden"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    // session hook 在 builtin 之后执行，覆盖了 builtin 的结果
    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                user_input: Some("original".to_string()),
                ..Default::default()
            },
            &[],
        )
        .unwrap();
    // session hook 在 builtin 之后执行，覆盖了 builtin 的结果
    assert_eq!(result.user_input.as_deref(), Some("session_overridden"));
}

#[test]
fn test_hook_manager_remove_session_hook() {
    let mut manager = HookManager::default();
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("echo test".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );
    assert_eq!(manager.list_hooks().len(), 1);

    assert!(manager.remove_session_hook(HookEvent::PreSendMessage, 0));
    assert!(manager.list_hooks().is_empty());

    // 移除不存在的索引
    assert!(!manager.remove_session_hook(HookEvent::PreSendMessage, 0));
}

#[test]
fn test_hook_chain_execution() {
    let mut manager = HookManager::default();

    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "first"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "second"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                user_input: Some("original".to_string()),
                ..Default::default()
            },
            &[],
        )
        .unwrap();

    // 最后一个 hook 的输出应该覆盖之前的
    assert_eq!(result.user_input.as_deref(), Some("second"));
}

#[test]
fn test_hook_stop_stops_chain() {
    let mut manager = HookManager::default();

    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("exit 1".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Stop,
            filter: HookFilter::default(),
        },
    );
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "should_not_reach"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                ..Default::default()
            },
            &[],
        )
        .unwrap();

    assert!(result.is_halt());
    assert!(result.user_input.is_none());
}

#[test]
fn test_builtin_hook_clone() {
    let mut manager = HookManager::default();
    manager.register_builtin(HookEvent::PreLlmRequest, "test_clone", |_| {
        Some(HookResult::default())
    });
    // HookManager 的 Clone 依赖 BuiltinHook 的 Clone（通过 Arc）
    let cloned = manager.clone();
    assert_eq!(cloned.list_hooks().len(), 1);
}

#[test]
fn test_on_error_skip_continues_chain() {
    // on_error=skip 时，第一个 hook 失败不影响后续 hook 执行
    let mut manager = HookManager::default();

    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("exit 1".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "survived"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                user_input: Some("original".to_string()),
                ..Default::default()
            },
            &[],
        )
        .unwrap();

    // 第二个 hook 应正常执行
    assert!(!result.is_halt());
    assert_eq!(result.user_input.as_deref(), Some("survived"));
}

#[test]
fn test_on_error_stop_stops_chain() {
    // on_error=stop 时，失败的 hook 中止整条链
    let mut manager = HookManager::default();

    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("exit 1".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Stop,
            filter: HookFilter::default(),
        },
    );
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "should_not_reach"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                ..Default::default()
            },
            &[],
        )
        .unwrap();

    assert!(result.is_halt());
    assert!(result.user_input.is_none());
}

#[test]
fn test_on_error_default_is_skip() {
    // HookDef 不设 on_error 时，YAML 反序列化应默认为 skip
    let yaml = r#"command: "exit 1"
timeout: 5"#;
    let def: HookDef = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(def.on_error, OnError::Skip);
}

#[test]
fn test_on_error_yaml_parsing() {
    // 验证 on_error 字段能正确从 YAML 反序列化
    let yaml_skip = r#"command: "echo test"
on_error: skip"#;
    let def: HookDef = serde_yaml::from_str(yaml_skip).unwrap();
    assert_eq!(def.on_error, OnError::Skip);

    let yaml_stop = r#"command: "echo test"
on_error: stop"#;
    let def: HookDef = serde_yaml::from_str(yaml_stop).unwrap();
    assert_eq!(def.on_error, OnError::Stop);
}

#[test]
fn test_shell_hook_stderr_captured() {
    // 验证 stderr 输出不会导致死锁，且进程正常完成
    let hook = ShellHook {
        name: None,
        command: r#"echo '{"user_input": "ok"}'; echo "debug info" >&2"#.to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("test".to_string()),
        ..Default::default()
    };
    let result = execute_shell_hook(&hook, &ctx).unwrap();
    assert_eq!(result.user_input.as_deref(), Some("ok"));
}

#[test]
fn test_shell_hook_stderr_in_error() {
    // 验证失败时 stderr 内容包含在错误信息中
    let hook = ShellHook {
        name: None,
        command: r#"echo "something went wrong" >&2; exit 1"#.to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext::default();
    let result = execute_shell_hook(&hook, &ctx);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("stderr:"), "错误信息应包含 stderr: {}", err);
    assert!(
        err.contains("something went wrong"),
        "错误信息应包含 stderr 内容: {}",
        err
    );
}

#[test]
fn test_hook_entry_session_index() {
    // 验证 list_hooks 为 session hook 返回正确的局部索引
    let mut manager = HookManager::default();

    // 注册 builtin hook（不应有 session_index）
    manager.register_builtin(HookEvent::PreSendMessage, "test", |_| None);

    // 注册两个 session hook
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("echo first".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("echo second".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Stop,
            filter: HookFilter::default(),
        },
    );

    let hooks = manager.list_hooks();
    assert_eq!(hooks.len(), 3);

    // builtin hook 无 session_index
    assert_eq!(hooks[0].source, "builtin");
    assert!(hooks[0].session_index.is_none());
    assert!(hooks[0].on_error.is_none());

    // 第一个 session hook
    assert_eq!(hooks[1].source, "session");
    assert_eq!(hooks[1].session_index, Some(0));
    assert_eq!(hooks[1].on_error, Some(OnError::Skip));

    // 第二个 session hook
    assert_eq!(hooks[2].source, "session");
    assert_eq!(hooks[2].session_index, Some(1));
    assert_eq!(hooks[2].on_error, Some(OnError::Stop));
}

#[test]
fn test_switch_model_field_removed() {
    // 验证旧脚本返回 _switch_model 字段时不会报错（serde 静默忽略未知字段）
    let json = r#"{"user_input": "test", "_switch_model": "gpt-4"}"#;
    let result: HookResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.user_input.as_deref(), Some("test"));
}

#[test]
fn test_new_hook_events_roundtrip() {
    // 验证新增的事件能正确序列化/反序列化
    for event in [
        HookEvent::Stop,
        HookEvent::PreMicroCompact,
        HookEvent::PostMicroCompact,
        HookEvent::PreAutoCompact,
        HookEvent::PostAutoCompact,
        HookEvent::PostToolExecutionFailure,
    ] {
        let s = event.as_str();
        let parsed = HookEvent::parse(s).unwrap();
        assert_eq!(event, parsed);
    }
}

#[test]
fn test_hook_result_retry_feedback() {
    // action=stop + retry_feedback
    let json = r#"{"action": "stop", "retry_feedback": "请修正敏感信息"}"#;
    let result: HookResult = serde_json::from_str(json).unwrap();
    assert!(result.is_stop());
    assert_eq!(result.retry_feedback.as_deref(), Some("请修正敏感信息"));
}

#[test]
fn test_hook_result_action_stop_with_retry_feedback() {
    // 新字段 action=stop + retry_feedback
    let json = r#"{"action": "stop", "retry_feedback": "请修正敏感信息"}"#;
    let result: HookResult = serde_json::from_str(json).unwrap();
    assert!(result.is_stop());
    assert_eq!(result.retry_feedback.as_deref(), Some("请修正敏感信息"));
}

#[test]
fn test_hook_result_additional_context() {
    let json = r#"{"additional_context": "必须保留宪法规则"}"#;
    let result: HookResult = serde_json::from_str(json).unwrap();
    assert_eq!(
        result.additional_context.as_deref(),
        Some("必须保留宪法规则")
    );
}

#[test]
fn test_hook_result_system_message() {
    let json = r#"{"system_message": "纠查官已审查"}"#;
    let result: HookResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.system_message.as_deref(), Some("纠查官已审查"));
}

#[test]
fn test_hook_result_tool_error() {
    let json = r#"{"tool_error": "权限不足"}"#;
    let result: HookResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.tool_error.as_deref(), Some("权限不足"));
}

#[test]
fn test_hook_context_new_fields() {
    let ctx = HookContext {
        event: HookEvent::PreAutoCompact,
        tool_error: None,
        ..Default::default()
    };
    let json = serde_json::to_string(&ctx).unwrap();
    assert!(json.contains("pre_auto_compact"));
    // skip_serializing_if 应跳过 None 字段
    assert!(!json.contains("tool_error"));
}

#[test]
fn test_hook_filter_tool_matcher() {
    let filter = HookFilter {
        tool_name: None,
        tool_matcher: Some("Shell".to_string()),
        model_prefix: None,
    };
    assert!(!filter.is_empty());

    // 匹配 Shell
    let ctx = HookContext {
        event: HookEvent::PreToolExecution,
        tool_name: Some("Shell".to_string()),
        ..Default::default()
    };
    assert!(filter.matches(&ctx));

    // 不匹配 Write
    let ctx = HookContext {
        event: HookEvent::PreToolExecution,
        tool_name: Some("Write".to_string()),
        ..Default::default()
    };
    assert!(!filter.matches(&ctx));

    // 上下文中没有 tool_name → 不匹配
    let ctx = HookContext {
        event: HookEvent::PreToolExecution,
        ..Default::default()
    };
    assert!(!filter.matches(&ctx));
}

#[test]
fn test_hook_filter_tool_name_priority_over_matcher() {
    // tool_name 精确匹配优先于 tool_matcher
    let filter = HookFilter {
        tool_name: Some("Shell".to_string()),
        tool_matcher: Some("Write|Edit".to_string()),
        model_prefix: None,
    };
    let ctx = HookContext {
        event: HookEvent::PreToolExecution,
        tool_name: Some("Write".to_string()),
        ..Default::default()
    };
    // tool_name 要求精确匹配 "Shell"，不匹配 "Write"
    assert!(!filter.matches(&ctx));
}

#[test]
fn test_hook_filter_tool_matcher_yaml() {
    let yaml = r#"tool_matcher: "Shell""#;
    let filter: HookFilter = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(filter.tool_matcher.as_deref(), Some("Shell"));
    assert!(filter.tool_name.is_none());
}

#[test]
fn test_render_prompt_template() {
    let template = "事件: {{event}}, 输入: {{user_input}}, 工具: {{tool_name}}";
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("hello".to_string()),
        tool_name: Some("Shell".to_string()),
        ..Default::default()
    };
    let rendered = render_prompt_template(template, &ctx);
    assert!(rendered.contains("pre_send_message"));
    assert!(rendered.contains("hello"));
    assert!(rendered.contains("Shell"));
}

#[test]
fn test_render_prompt_template_empty_fields() {
    let template = "输入: {{user_input}}, 输出: {{assistant_output}}";
    let ctx = HookContext::default();
    let rendered = render_prompt_template(template, &ctx);
    assert_eq!(rendered, "输入: , 输出: ");
}

#[test]
fn test_extract_json_from_llm_output() {
    // 纯 JSON
    assert_eq!(
        extract_json_from_llm_output(r#"{"user_input": "test"}"#),
        Some(r#"{"user_input": "test"}"#)
    );

    // JSON 包裹在 markdown 中
    assert_eq!(
        extract_json_from_llm_output("```json\n{\"user_input\": \"test\"}\n```"),
        Some(r#"{"user_input": "test"}"#)
    );

    // JSON 前有文本
    assert_eq!(
        extract_json_from_llm_output("Here is the result: {\"action\": \"stop\"}"),
        Some(r#"{"action": "stop"}"#)
    );

    // 无 JSON
    assert_eq!(extract_json_from_llm_output("no json here"), None);
}

#[test]
fn test_hook_type_yaml_parsing() {
    let yaml_bash = r#"command: "echo hello""#;
    let def: HookDef = serde_yaml::from_str(yaml_bash).unwrap();
    assert_eq!(def.r#type, HookType::Bash);

    let yaml_llm = r#"
type: llm
prompt: "check this""#;
    let def: HookDef = serde_yaml::from_str(yaml_llm).unwrap();
    assert_eq!(def.r#type, HookType::Llm);
    assert_eq!(def.prompt.as_deref(), Some("check this"));
}

#[test]
fn test_hook_def_bash_missing_command() {
    let def = HookDef {
        r#type: HookType::Bash,
        command: None,
        prompt: None,
        model: None,
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
    };
    assert!(def.into_hook_kind().is_err());
}

#[test]
fn test_hook_def_llm_missing_prompt() {
    let def = HookDef {
        r#type: HookType::Llm,
        command: None,
        prompt: None,
        model: None,
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
    };
    assert!(def.into_hook_kind().is_err());
}

#[test]
fn test_hook_type_display() {
    assert_eq!(format!("{}", HookType::Bash), "bash");
    assert_eq!(format!("{}", HookType::Llm), "llm");
}

#[test]
fn test_hook_entry_hook_type() {
    let mut manager = HookManager::default();

    // builtin hook → hook_type = "builtin"
    manager.register_builtin(HookEvent::PreSendMessage, "test", |_| None);

    // bash session hook → hook_type = "bash"
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("echo test".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    // llm session hook → hook_type = "llm"
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Llm,
            command: None,
            prompt: Some("check content".to_string()),
            model: None,
            timeout: 30,
            retry: 1,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let hooks = manager.list_hooks();
    assert_eq!(hooks.len(), 3);
    assert_eq!(hooks[0].hook_type, "builtin");
    assert_eq!(hooks[1].hook_type, "bash");
    assert_eq!(hooks[2].hook_type, "llm");
}

#[test]
fn test_llm_hook_no_provider_returns_err() {
    let hook = LlmHook {
        name: None,
        prompt: "test".to_string(),
        model: None,
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext::default();
    let result = execute_llm_hook(&hook, &ctx, &None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("未注入 provider"));
}
