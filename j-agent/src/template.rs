//! 模板资源加载
//!
//! 从编译时嵌入的资源中加载系统 prompt 模板。
//! 使用 `include_str!` 替代 `rust-embed` 以减少依赖。

/// Teammate system prompt 模板
pub fn teammate_system_prompt_template() -> &'static str {
    include_str!("../assets/teammate_system_prompt.md")
}

/// SubAgent system prompt 模板
pub fn sub_agent_system_prompt_template() -> &'static str {
    include_str!("../assets/sub_agent_system_prompt.md")
}

/// 默认系统提示词模板
pub fn default_system_prompt() -> &'static str {
    include_str!("../assets/system_prompt_default.md")
}

/// 默认记忆占位文件
pub fn default_memory() -> &'static str {
    include_str!("../assets/memory_default.md")
}

/// 默认灵魂占位文件
pub fn default_soul() -> &'static str {
    include_str!("../assets/soul_default.md")
}

/// 默认 AGENTS.md 模板
pub fn default_agent_md() -> &'static str {
    include_str!("../assets/agent_md_default.md")
}
