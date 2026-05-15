# AI 配置

首次运行 `j chat` 时，若尚未配置模型提供方，会自动进入内置配置界面完成初始配置。已有配置后，也可随时在对话界面中按 **Ctrl+E** 或输入 `/config` 重新编辑。

配置文件路径: `~/.jdata/agent/data/agent_config.json`（也可手动编辑）

```json
{
  "providers": [
    {
      "name": "GPT-4o",
      "api_base": "https://api.openai.com/v1",
      "api_key": "sk-your-api-key",
      "model": "gpt-4o",
      "supports_vision": true
    }
  ],
  "active_index": 0,
  "max_history_messages": 20,
  "max_context_tokens": 100000,
  "theme": "midnight",
  "tools_enabled": true,
  "max_tool_rounds": 10,
  "tool_confirm_timeout": 0,
  "auto_restore_session": false
}
```

> 支持配置多个模型提供方，可在对话中切换

## 配置界面

按 `Ctrl+E` 或输入 `/config` 进入可视化配置界面。当前界面包含 `Model`、`Session`、`Global`、`Tools`、`Skills`、`Hooks`、`Commands`、`Teammates`、`Archive` 九个 Tab。

| 按键 | 功能 |
|------|------|
| `←` / `→` | 切换 Tab |
| `↑` / `↓` / `j` / `k` | 在当前列表中移动 |
| `Enter` | 编辑字段 / 执行当前项动作 |
| `Esc` | 保存配置并返回对话 |

**Model Tab**：`Tab`/`Shift+Tab` 切换 Provider，`a` 新增，`d` 删除，`s` 设为活跃

**Tools / Skills Tab**：`Enter`/`空格` 启用禁用，`a` 全部启用，`d` 全部禁用，`t` 切换总开关（仅 Tools）

**Session / Archive / Teammates Tab**：
- `Session`：`Enter` 恢复，`d` 删除，`n` 新建
- `Archive`：`Enter` 还原，`d` 删除
- `Teammates`：`Enter` 查看状态，`s` 停止