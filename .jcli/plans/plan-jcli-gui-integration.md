# Plan: jcli-gui-integration

## 结论

建议在当前 `jcli` 仓库内从头集成一个 Tauri GUI 应用，但不建议直接把现有 `assets/reader/` 改造成 Tauri 主应用。

推荐方向：

- 保留当前 `j read` 的轻量浏览器 Reader，继续用于 CLI 场景、远程场景、无 GUI 环境。
- 新增一个同仓库 GUI app，作为 jcli 的桌面版入口。
- 当前 Reader 能力迁移/复用为 GUI 的一个 feature，而不是让 Reader 自身膨胀成整个 GUI。
- 之前的 `../jgui/` 作为经验和素材来源，只摘取架构、IPC、窗口生命周期、设置、快捷键、组件体系等可复用部分，不整包搬迁。

## 推荐目录结构

第一阶段采用低侵入结构：

```text
jcli/
  src/                         # 现有 CLI 主体，第一阶段尽量不动
  assets/reader/               # 当前轻量浏览器 Reader，保留
  apps/
    gui/
      package.json
      vite.config.ts
      tsconfig.json
      src/
      src-tauri/
```

后续如果 GUI 与 CLI 共享逻辑越来越多，再考虑中期结构调整：

```text
jcli/
  crates/
    jcli-core/                 # 配置、数据目录、别名、日报、todo、reader helper 等共享逻辑
    jcli-agent/                # Agent runtime / shared types，可选
    jcli-tools/                # 工具调用/skill/hook/permission 等可选
  cli/                         # 或保留现有 src/ 作为 CLI binary
  apps/
    gui/
```

第一阶段不建议一上来就大拆 crate，避免把 GUI 起步和核心重构绑死。

## CLI 入口建议

新增桌面 GUI 入口：

```bash
j gui
j gui .
j gui --workspace ~/project
```

当前轻量 Reader 保留：

```bash
j read .
```

后续也可以增加：

```bash
j read --gui .
```

但不建议第一阶段就改变 `j read` 的默认行为。

## 与当前 Reader 的关系

当前 `assets/reader/` 继续作为轻量 Reader：

- 浏览器/本地 HTTP server 模式。
- 适合快速打开目录或文件。
- 适合无 Tauri 环境或不想打开桌面应用的场景。

新 GUI 中新增 Reader feature：

```text
apps/gui/src/features/reader/
```

优先复用当前 Reader 已验证过的交互和能力：

- 文件树
- 打开文件
- 保存文件
- 创建文件/文件夹
- Markdown / text / code 查看编辑
- JSON 查看器
- Diff 工具
- 设置菜单
- 工具箱

但通信方式从 HTTP API 改为 Tauri IPC：

```text
当前 Reader:
React -> fetch('./api/...') -> Rust local server

GUI Reader:
React -> invoke(...) / Channel -> Tauri Rust backend
```

## 是否继续在当前 Reader 做终端

建议暂停当前浏览器 Reader 的 WebSocket PTY 终端实现。

原因：

- 如果要做 GUI，PTY 终端更适合在 Tauri 内用 IPC/Channel 实现。
- 否则会出现两套终端协议：
  - 浏览器 Reader: WebSocket + PTY
  - Tauri GUI: IPC/Channel + PTY
- 终端属于 GUI 核心能力，应该优先放在新 GUI 中。

当前 Reader 可继续做小 UI 修正：

- 设置菜单不再只是主题设置。
- 点击外部/按 Esc 关闭设置菜单。
- 工具箱顶部删除冗余“工具箱”标题。
- 编辑区滚动条放到最右侧。

## 从 `../jgui/` 复用什么

建议复用/参考：

1. Tauri v2 skeleton
   - `src-tauri/tauri.conf.json`
   - 窗口尺寸
   - titleBarStyle
   - plugin 配置

2. IPC 模式
   - Rust commands
   - Tauri Channel 流式通信
   - Events 做全局通知
   - 前端 `ipc.ts` 封装

3. 窗口生命周期
   - window state
   - tray
   - close-to-tray
   - dock badge

4. 设置系统
   - settings command
   - GUI 自有设置持久化
   - jcli 数据路径展示

5. 快捷键
   - `tauri-plugin-global-shortcut`
   - 前端快捷键 registry
   - `Command + J` 终端切换

6. UI 基础
   - Radix / shadcn 风格组件
   - Tooltip / Dialog / Popover / Tabs / ScrollArea
   - Jotai 状态管理经验

7. Chat / Agent 流式架构经验
   - Channel 推流
   - 前端全局 listener
   - 运行时状态隔离

不建议直接搬迁：

1. 整个旧 `jgui` 前端状态体系。
2. 旧 `jgui` 的全部 CodeStable 约束。
3. 旧版 `j-cli = "12.10.44"` crates.io 依赖方式。
4. 过重的 Chat/Agent UI。
5. 与“独立 jgui 仓库”相关的解耦规则。

## GUI MVP 范围

第一版 GUI 不追求完整复制旧 jgui，也不追求一次覆盖全部 jcli 能力。

建议 MVP：

```text
J GUI
  ├─ Explorer / Reader
  ├─ Terminal
  ├─ Tools
  ├─ Settings
  └─ Chat / Agent 入口占位或最小接入
```

### P0: 桌面壳

目标：GUI 能在当前仓库中独立启动。

内容：

- `apps/gui/` Tauri v2 + React + Vite 项目初始化。
- 基础窗口配置。
- 基础主题。
- ActivityBar + 主内容区 + 底部 panel 布局。
- `j gui` 命令能启动 GUI。
- 开发命令和构建命令接入仓库 Makefile 或 npm script。

验收：

- `j gui` 能打开桌面窗口。
- `apps/gui` 前端能 build。
- Tauri dev 能启动。

### P1: Reader feature

目标：把当前 Reader 的核心能力作为 GUI 的一个模块落地。

内容：

- 文件树。
- 打开文件。
- 保存文件。
- 创建文件/文件夹。
- 基础编辑/查看区域。
- JSON 查看器。
- Diff 工具。
- 工具箱 tab。
- 设置菜单。

通信方式：

- 前端通过 `invoke` 调用 Rust command。
- 文件读取/保存等先用 Tauri command，不走 HTTP server。

验收：

- `j gui .` 打开指定 workspace。
- 能浏览目录、打开文件、保存文件。
- 能创建文件/文件夹。
- 工具箱可用。

### P2: Terminal feature

目标：实现 VS Code 风格底部终端。

内容：

- xterm.js。
- `@xterm/addon-fit`。
- `@xterm/addon-webgl`，失败 fallback。
- `Command + J` / `Ctrl + J` 切换底部终端。
- PTY 后端使用 `portable-pty`。
- 输出流建议走 Tauri Channel。
- 输入、resize、kill 通过 Tauri command。

推荐数据流：

```text
xterm onData
  -> invoke("terminal_write", { terminalId, data })
  -> portable-pty stdin

portable-pty stdout
  -> Channel<TerminalEvent>
  -> xterm.write(data)

xterm fit resize
  -> invoke("terminal_resize", { terminalId, cols, rows })
```

验收：

- `Command + J` 打开/关闭底部终端。
- 能执行 shell 命令。
- resize 后终端尺寸正确。
- 关闭窗口/关闭 terminal 后子进程被清理。
- WebGL 可用时启用，不可用时 fallback。

### P3: Settings feature

目标：形成 GUI 级设置系统。

内容：

- 主题。
- 字体大小。
- Terminal 字体。
- 默认 workspace。
- jcli 数据目录展示。
- Agent 配置入口。
- GUI 设置单独持久化，jcli 数据仍走 `~/.jdata/`。

验收：

- 设置修改后持久化。
- 重启 GUI 后恢复。
- 不破坏 jcli CLI 配置。

### P4: Chat / Agent 最小接入

目标：GUI 开始成为 jcli 的桌面版本，而不只是 Reader。

内容：

- 先接配置读取。
- 再接 Chat session 列表。
- 然后接单轮/流式 Chat。
- Agent 后续再接，避免一次性复杂化。

验收：

- GUI 能读取 jcli agent/chat 相关配置。
- 能展示已有会话或创建最小会话。
- 流式输出稳定后再进入 Agent。

## 后端共享逻辑路线

### 短期

GUI 的 Rust 后端先在 `apps/gui/src-tauri` 内实现最小 command：

- 文件系统 reader commands。
- terminal commands。
- settings commands。

如果需要引用 jcli 数据目录、配置路径等，可以优先复用常量或复制极少量稳定逻辑，但避免大范围依赖当前 binary 内部模块。

### 中期

逐步抽出 `crates/jcli-core`：

- config path。
- data path。
- alias 数据模型。
- report 数据模型。
- todo 数据模型。
- reader 文件 helper。
- chat/agent config shared types。

CLI 和 GUI 都依赖 `jcli-core`。

### 长期

视情况继续拆：

- `jcli-agent`
- `jcli-tools`
- `jcli-skill`
- `jcli-chat`

但只有当复用压力真的出现时再拆，不预先过度设计。

## 安全与权限边界

GUI 是桌面应用，文件系统和终端能力都更敏感。

需要明确：

- Reader 文件操作默认限制在当前 workspace。
- Terminal cwd 默认是 workspace root。
- 不允许前端传任意路径绕过 workspace 限制，除非用户通过文件选择器明确授权。
- Tauri capability / plugin permission 要最小化。
- PTY 子进程生命周期必须受 TerminalManager 管理，窗口关闭时清理。
- 不在 TUI/GUI 后端使用无约束 stdout/stderr 日志污染界面。

## 与当前仓库构建体系的关系

第一阶段建议新增独立命令，不干扰现有 `cargo fmt` / `cargo clippy`：

```bash
make gui-dev
make gui-build
make gui-check
```

或在根 `package.json` / `Makefile` 增加对应入口。

最终 CI 可分阶段接入：

1. 先只检查 GUI 前端 build。
2. 再检查 Tauri cargo check。
3. 最后增加完整 bundle 构建。

不要第一天就要求三平台 Tauri 打包全绿，否则起步成本过高。

## 风险

1. 当前 jcli 是 CLI binary 为主，GUI 想直接复用内部逻辑会受限。
2. Tauri 引入后发布链路复杂度会上升。
3. 如果过早搬旧 jgui，容易把旧项目复杂状态体系也搬进来。
4. PTY terminal 涉及跨线程、进程清理、流式输出，需要单独测试。
5. Chat/Agent 状态机复杂，不应和 GUI skeleton 同时大规模接入。

## 推荐执行顺序

1. 创建 `apps/gui` Tauri skeleton。
2. 接入根仓库开发/构建命令。
3. 实现 GUI 基础 layout。
4. 实现 Reader 文件树 + 打开/保存文件。
5. 迁移工具箱核心能力。
6. 实现 Terminal。
7. 实现 Settings。
8. 最小接入 Chat。
9. 最小接入 Agent。
10. 评估是否抽 `crates/jcli-core`。

## 当前 Reader 待办处理建议

在 GUI 计划启动前，当前 Reader 仍可完成小修：

- 设置菜单更像 VS Code。
- 点击外部关闭设置菜单。
- 工具箱顶部去掉冗余标题。
- 文件滚动条放到编辑区域最右侧。

但不建议继续在当前 Reader 内实现 WebSocket PTY terminal。

## 最终目标

形成两个互补入口：

```bash
j read .      # 轻量 Reader，快速、浏览器模式、兼容远程/无 GUI 场景
j gui .       # 完整桌面 GUI，包含 Reader、Terminal、Tools、Settings、Chat、Agent
```

这样既保留 jcli 的 CLI 生产力属性，又给长期桌面化留出清晰空间。
