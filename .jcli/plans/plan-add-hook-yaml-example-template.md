# 添加 HOOK.yaml 模板文件到 .jcli/hooks/example/

## 背景

当用户首次使用 `jcli ai` 的 hook 功能时，`.jcli/hooks/` 目录初始化时没有提供模板文件，导致用户需要查阅文档才能正确编写 `HOOK.yaml`。为了帮助用户高效编写 hook，需要在初始化时提供一个默认的模板文件。

## 当前状态

- 用户级 hooks 目录：`~/.jdata/agent/hooks/`（由 `hooks_dir()` 函数创建）
- 项目级 hooks 目录：`.jcli/hooks/`（由 `project_hooks_dir()` 查找，不自动创建）
- `.jcli/` 目录由 `ensure_config_dir()` 创建（在 `permission/rules.rs`）
- `HOOK.yaml` 结构定义在 `HookDirDef` 结构体中（`src/command/chat/infra/hook.rs`）
- 帮助文档：`assets/help/hook.md` 包含完整的 hook 使用说明和示例

## 实现方案

### 修改位置

1. `src/command/chat/permission/rules.rs` - 修改 `ensure_config_dir()` 函数
2. `src/command/chat/infra/hook.rs` - 修改 `load_hooks_from_dir()` 函数，跳过 `example` 目录
3. `assets/hook_yaml_example.yaml` - 新增模板文件

### 实现内容

1. 在 `ensure_config_dir()` 创建 `.jcli/` 目录后，同时创建 `.jcli/hooks/example/` 目录和 `HOOK.yaml` 模板文件
2. 在 `load_hooks_from_dir()` 中跳过名为 `example` 的目录（因为它是模板示例，不是实际可执行的 hook）
3. 模板文件使用内嵌字符串（include_str!）方式打包到二进制中

### 文件位置

模板文件创建在：`.jcli/hooks/example/HOOK.yaml`

用户可以直接参考这个示例目录结构和配置格式来创建自己的 hook。

### 代码改动

#### 1. `src/command/chat/permission/rules.rs`

```rust
// ensure_config_dir() 函数改动

pub fn ensure_config_dir() -> Option<PathBuf> {
    let dir = std::env::current_dir().ok()?.join(".jcli");
    let _ = std::fs::create_dir_all(&dir);
    
    // 创建 hooks/example 目录和 HOOK.yaml 模板（仅在首次创建时）
    let hooks_dir = dir.join("hooks");
    let example_dir = hooks_dir.join("example");
    if !example_dir.exists() {
        let _ = std::fs::create_dir_all(&example_dir);
        let example_yaml = example_dir.join("HOOK.yaml");
        if !example_yaml.exists() {
            const HOOK_YAML_EXAMPLE: &str = include_str!("../../assets/hook_yaml_example.yaml");
            if let Err(e) = std::fs::write(&example_yaml, HOOK_YAML_EXAMPLE) {
                // 静默失败，不影响主流程
                eprintln!("无法写入 hook 模板文件: {}", e);
            }
        }
    }
    
    Some(dir)
}
```

#### 2. `src/command/chat/infra/hook.rs`

```rust
// load_hooks_from_dir() 函数改动（跳过 example 目录）

fn load_hooks_from_dir(dir: &Path, source_name: &str) -> Vec<(String, HookDirDef, PathBuf)> {
    let mut hooks = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return hooks,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let hook_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        // 跳过 example 目录（模板示例，不是实际 hook）
        if hook_name == "example" {
            continue;
        }
        
        let hook_yaml = path.join("HOOK.yaml");
        if !hook_yaml.exists() {
            continue;
        }
        // ... 后续代码不变
    }
    // ... 后续代码不变
}
```

### 新增资源文件

创建 `assets/hook_yaml_example.yaml`，包含完整的模板内容和注释。

## 文件清单

| 文件 | 操作 |
|------|------|
| `src/command/chat/permission/rules.rs` | 修改 `ensure_config_dir()` 函数 |
| `src/command/chat/infra/hook.rs` | 修改 `load_hooks_from_dir()` 函数，跳过 example 目录 |
| `assets/hook_yaml_example.yaml` | 新增模板内容文件 |

## 模板文件预览

```yaml
# HOOK.yaml - Hook 配置示例文件
#
# 此文件为模板示例，展示了 HOOK.yaml 的完整配置格式。
# 创建新 hook 时，复制此目录结构：
#   .jcli/hooks/<your_hook_name>/HOOK.yaml
#
# Hook 目录结构：
# .jcli/hooks/<hook_name>/
# ├── HOOK.yaml      # hook 定义（必须）
# └── script.sh      # 可选脚本（bash hook 可直接用文件名调用）

# ============================================
# 字段说明
# ============================================

# events: [必填] 绑定的事件列表，一个 hook 可绑定多个事件
# 可用事件：
#   - pre_send_message        # 用户发送消息前（可修改 user_input）
#   - post_send_message       # 用户发送消息后（仅通知）
#   - pre_llm_request         # LLM API 请求前（可修改 messages、system_prompt）
#   - post_llm_response       # LLM 回复完成后（可修改 assistant_output）
#   - pre_tool_execution      # 工具执行前（可修改 tool_arguments，action=skip）
#   - post_tool_execution     # 工具执行成功后（可修改 tool_result）
#   - post_tool_execution_failure # 工具执行失败后（可修改 tool_error）
#   - stop                    # LLM 即将结束回复
#   - pre_micro_compact       # 轮次级压缩前
#   - post_micro_compact      # 轮次级压缩后
#   - pre_auto_compact        # 全量压缩前
#   - post_auto_compact       # 全量压缩后
#   - session_start           # 会话启动时
#   - session_end             # 会话退出时

# type: [可选] hook 类型，默认 bash
#   - bash: 通过 sh -c 执行 Shell 命令
#   - llm:  通过 prompt 模板调用 LLM

# command: [type=bash 时必填] Shell 命令
#   - 相对路径脚本以 hook 目录为 cwd，可直接用文件名调用

# prompt: [type=llm 时必填] LLM prompt 模板
#   - 支持 {{variable}} 模板变量：
#     {{event}}, {{user_input}}, {{assistant_output}},
#     {{tool_name}}, {{tool_arguments}}, {{tool_result}},
#     {{model}}, {{cwd}}

# timeout: [可选] 超时秒数，bash 默认 10，llm 默认 30

# retry: [可选] 重试次数，默认 0（bash）/ 1（llm）

# on_error: [可选] 失败策略，默认 skip
#   - skip:  记录日志继续执行后续 hook
#   - abort: 中止整条 hook 链

# filter: [可选] 条件过滤，仅当匹配时执行
#   - tool_name: 工具名精确匹配
#   - tool_matcher: 工具名模式匹配（管道分隔，如 "Bash|Write|Edit"）
#   - model_prefix: 模型名前缀匹配

# ============================================
# 示例配置（取消注释即可使用）
# ============================================

# --- 示例 1：Bash hook - 基础配置 ---
# events: [pre_send_message]
# type: bash
# command: script.sh  # 脚本放在 hook 目录下，直接用文件名调用
# timeout: 5
# on_error: skip

# --- 示例 2：LLM hook - AI 回复纠查官 ---
# events: [post_llm_response]
# type: llm
# prompt: |
#   检查以下 AI 回复是否包含敏感信息（密码、密钥、token）：
#   {{assistant_output}}
#   如果包含敏感信息，返回 {"action":"stop","retry_feedback":"请移除敏感信息"}。
#   如果没有问题，返回空 JSON {}。
# timeout: 30
# retry: 1
# on_error: skip

# --- 示例 3：多事件绑定 ---
# events: [pre_send_message, post_send_message]
# type: bash
# command: log.sh
# timeout: 5

# --- 示例 4：带过滤器的工具审查 ---
# events: [pre_tool_execution]
# type: llm
# prompt: |
#   审查工具调用是否安全：工具={{tool_name}}, 参数={{tool_arguments}}
#   如果不安全，返回 {"action":"skip"}。
#   如果安全，返回 {}。
# filter:
#   tool_matcher: "Bash|Shell"
# timeout: 15
# retry: 1

# ============================================
# HookResult JSON 字段参考（脚本 stdout 返回）
# ============================================
#
# 返回的 JSON 只包含要修改的字段，空 {} 表示无修改：
#
# | 字段               | 生效事件                              | 说明 |
# |--------------------|---------------------------------------|------|
# | user_input         | pre_send_message                      | 替换用户消息 |
# | assistant_output   | post_llm_response                     | 替换 AI 回复 |
# | messages           | pre_llm_request, post_*_compact       | 替换消息列表 |
# | system_prompt      | pre_llm_request                       | 替换系统提示词 |
# | tool_arguments     | pre_tool_execution                    | 替换工具参数 |
# | tool_result        | post_tool_execution                   | 替换工具结果 |
# | tool_error         | post_tool_execution_failure           | 替换错误信息 |
# | inject_messages    | pre_llm_request                       | 追加消息到末尾 |
# | retry_feedback     | pre*/stop/post_llm_response           | 带反馈重试 |
# | additional_context | pre_llm_request, stop, pre_auto_compact | 追加到 system_prompt |
# | system_message     | 所有事件                              | 展示给用户的提示 |
# | action             | 大部分事件                            | "stop" 中止 / "skip" 跳过 |
#
# 完整文档请运行：jcli ai help hook
```

## 验证步骤

1. 运行 `cargo fmt` 格式化代码
2. 运行 `cargo clippy` 检查无告警
3. 运行 `cargo build` 确保编译通过
4. 删除 `.jcli/hooks/example/` 目录（如果已存在）
5. 触发 `ensure_config_dir()` 调用（如添加权限规则或进入 plan mode）
6. 检查 `.jcli/hooks/example/HOOK.yaml` 文件是否正确创建
7. 验证 example 目录不会被加载为实际 hook（在 hook list 中不显示）