use std::borrow::Cow;

use super::Assets;

/// 版本信息模板
///
/// 用途: `j version` 命令输出
/// 占位符: `{version}`, `{os}`, `{extra}`
/// 格式: Markdown 表格
pub fn version_template() -> Cow<'static, str> {
    Assets::get("version.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 默认系统提示词模板
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/system_prompt.md`
/// 占位符: `{{.tools}}`, `{{.skills}}`, `{{.style}}`, `{{.memory}}`, `{{.soul}}`
/// 格式: Markdown
pub fn default_system_prompt() -> Cow<'static, str> {
    Assets::get("system_prompt_default.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 默认记忆占位文件
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/memory.md`
/// 格式: Markdown
pub fn default_memory() -> Cow<'static, str> {
    Assets::get("memory_default.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 默认灵魂占位文件
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/soul.md`
/// 格式: Markdown
pub fn default_soul() -> Cow<'static, str> {
    Assets::get("soul_default.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 默认 AGENTS.md 模板
///
/// 用途: 首次运行时写入 `~/.jdata/agent/AGENTS.md`
/// 格式: Markdown
pub fn default_agent_md() -> Cow<'static, str> {
    Assets::get("agent_md_default.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

// Note: teammate_system_prompt_template and sub_agent_system_prompt_template
// are now provided by j_cli_core::template (using include_str!). Removed from here.

/// 诗句语录文本
///
/// 用途: Chat UI 欢迎框随机展示一句诗句
/// 格式: 纯文本，每行一句
///
/// 使用 `include_str!` 而非 `include_dir!` 的 `Assets::get`，
/// 因为 proc macro 增量编译缓存可能导致新增文件不被重新扫描。
pub fn quotes_text() -> &'static str {
    include_str!("../../assets/quotes.txt")
}
