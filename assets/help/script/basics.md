## 脚本 & 倒计时

| 命令 | 说明 |
|------|------|
| `j script <name> "<content>"` | 创建脚本并注册为别名（保存到 `~/.jdata/scripts/`） |
| `j script <name>` | 打开 TUI 编辑器创建或编辑脚本 |
| `j <script> [args...]` | 在当前终端执行脚本 |
| `j <script> -w [args...]` | 在**新终端窗口**中执行脚本 |
| `j time countdown <duration>` | 启动倒计时（支持 `30s` / `5m` / `1h`，不带单位默认按分钟） |
