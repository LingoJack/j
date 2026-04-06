# Plan: j-cli GUI 版本 (Tauri) 项目规划

## 一、项目概述

### 目标
为 j-cli 开发一个 macOS 原生风格的 GUI 版本，核心特性：
- 聚焦搜索界面（类似 Spotlight/Alfred/Raycast）
- 全局快捷键 `Cmd + J` 唤起，随用随到
- 系统托盘整合（解决托盘过多问题）
- 复用现有 j-cli 核心逻辑

### 技术栈
- **Tauri 2.x**: Rust 后端 + Web 前端
- **前端框架**: React 19 + TypeScript + Tailwind CSS v4
- **状态管理**: Zustand
- **UI 组件**: shadcn/ui 或自定义组件
- **构建工具**: Vite 6.x

---

## 二、代码复用性分析（关键）

### Breaking Change 风险评估

**关键结论**: 采用 **适配器模式**，可以 **零 Breaking Change**。

#### 现有代码调用链分析

```
main.rs -> cli.rs -> command/handler.rs -> command/*.rs (具体命令)
                                    ↓
                              config::YamlConfig
```

#### 零 Breaking Change 策略

**核心原则**: 只增不改，适配器封装

```
┌─────────────────────────────────────────────────────────────┐
│                     安全重构策略                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  现有 CLI 路径（保持不变）                                   │
│  ────────────────────                                       │
│  main.rs -> cli.rs -> handler.rs -> command/open.rs         │
│                                      └── handle_open()      │
│                                           ↓                 │
│                                      info!/error! 输出       │
│                                                             │
│  新增 GUI 路径（独立添加）                                   │
│  ────────────────────                                       │
│  Tauri Command -> core/open.rs                              │
│                    └── open_alias_silent()                  │
│                         ↓                                   │
│                    command/open.rs (复用逻辑)               │
│                    或直接实现（避免终端依赖）                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 具体方案对比

| 方案 | Breaking Change 风险 | 说明 |
|------|---------------------|------|
| ❌ 方案 A: 修改现有函数签名 | **高** | 改动 handle_open() 参数/返回值，影响 CLI |
| ❌ 方案 B: 抽取核心函数，修改原函数调用 | **中** | 需要修改现有 command/*.rs 文件 |
| ✅ **方案 C: 新增 silent 版本函数** | **无** | 只新增，不修改现有代码 |

#### 推荐方案 C: 新增 silent 版本

```rust
// ============ 现有代码（完全不变）============
// src/command/open.rs

/// CLI 入口，保持不变
pub fn handle_open(args: &[String], config: &YamlConfig) {
    // 现有实现完全不变
    let alias = &args[0];
    match do_open(alias, args, config) {
        Ok(msg) => info!("{}", msg),
        Err(e) => error!("{}", e),
    }
}

// 内部实现（保持不变）
fn do_open(alias: &str, args: &[String], config: &YamlConfig) -> Result<String, String> {
    // 现有逻辑...
}

// ============ 新增代码（仅添加）============
// src/core/open.rs（新文件）

/// GUI 友好版本：静默执行，返回结果
/// 注意：这是新增函数，不修改任何现有代码
pub fn open_alias_silent(alias: &str, args: &[String]) -> Result<String, String> {
    let config = YamlConfig::load();
    // 直接调用现有的内部实现
    crate::command::open::do_open(alias, args, &config)
}
```

**关键点**：
1. `core/` 模块是 **新增** 的，不修改现有 `command/` 模块
2. 现有 `handle_open()` 保持完全不变
3. 只需要将现有的私有函数 `do_open()` 改为 `pub(crate)`（模块内可见）
4. 或者：core 模块直接复制逻辑（代码略有重复，但最安全）

#### 需要的小调整（非 Breaking）

| 调整 | 影响 | 风险 |
|------|------|------|
| 将内部函数改为 `pub(crate)` | 仅模块可见性 | 无 |
| 新增 `src/lib.rs` | 暴露公共模块 | 无 |
| 新增 `src/core/` 目录 | 新增代码 | 无 |

#### 验证清单

- [ ] `cargo build` 通过（CLI 正常编译）
- [ ] `j open chrome` 行为不变
- [ ] `j report test` 行为不变
- [ ] 所有现有测试通过

### 现有代码结构分析

经过详细代码审查，现有 j-cli 代码 **高度可复用**：

#### 可直接复用的模块（无需修改）

| 模块 | 路径 | 复用方式 | 说明 |
|------|------|----------|------|
| **YamlConfig** | `src/config/yaml_config.rs` | 直接引用 | 配置加载/保存/查询，完全独立 |
| **constants** | `src/constants.rs` | 直接引用 | 所有常量定义 |
| **util** | `src/util/` | 直接引用 | 工具函数（模糊匹配、文本处理等） |
| **alias 核心逻辑** | `src/command/alias.rs` | 抽取核心函数 | 别名 CRUD 操作 |
| **open 核心逻辑** | `src/command/open.rs` | 抽取核心函数 | 应用/URL/脚本启动 |

#### 需要适配的模块（轻量封装）

| 模块 | 原始依赖 | 适配方案 |
|------|----------|----------|
| **report** | TUI 编辑器、终端输出 | 抽取 `write_to_report()` 核心逻辑，GUI 直接调用 |
| **script** | 终端执行 | 通过 Tauri Command 调用，结果通过 IPC 返回 |
| **fuzzy 搜索** | 终端渲染 | 核心算法 `fuzzy::fuzzy_match()` 直接复用 |

#### 不适用 GUI 的模块（保留 CLI 专用）

| 模块 | 原因 |
|------|------|
| `src/tui/` | 终端 UI 组件，GUI 使用 React 替代 |
| `src/interactive/` | 终端交互式输入，GUI 使用 Web 组件替代 |
| `src/command/chat/` | 复杂 TUI Chat 界面，GUI 可选择性移植 |

### 代码复用策略

#### Step 1: 创建 lib.rs 暴露公共 API

```rust
// src/lib.rs (新增)
//! j-cli 核心库，供 CLI 和 GUI 共同使用

pub mod command;
pub mod config;
pub mod constants;
pub mod util;

// 重导出核心类型
pub use config::YamlConfig;
pub use constants::*;

// 暴露核心功能接口
pub mod core {
    //! GUI 友好的核心接口（无终端依赖）
    
    use crate::config::YamlConfig;
    
    /// 打开别名对应的应用/路径
    pub fn open_alias(alias: &str, args: &[String]) -> Result<(), String> {
        let config = YamlConfig::load();
        // 调用 command::open 的核心逻辑（去除 info!/error! 宏）
        ...
    }
    
    /// 搜索别名（模糊匹配）
    pub fn search_aliases(query: &str, config: &YamlConfig) -> Vec<SearchResult> {
        ...
    }
    
    /// 写入日报
    pub fn write_report(content: &str) -> Result<(), String> {
        ...
    }
}
```

#### Step 2: 分离终端依赖

现有代码中使用 `info!` / `error!` 宏输出到终端，需要改造：

```rust
// 方案 A: 使用 trait 抽象输出
pub trait Output {
    fn info(&self, msg: &str);
    fn error(&self, msg: &str);
}

// CLI 实现
struct TerminalOutput;
impl Output for TerminalOutput {
    fn info(&self, msg: &str) { println!("{}", msg); }
    fn error(&self, msg: &str) { eprintln!("{}", msg); }
}

// GUI 实现（返回 Result）
// GUI 直接使用 Result<String, String>，不依赖输出

// 方案 B（推荐）: 核心逻辑返回 Result，CLI 层包装输出
// src/command/open_core.rs（新增）
pub fn open_alias_core(alias: &str, args: &[String], config: &YamlConfig) -> Result<String, String> {
    // 纯逻辑，返回结果
}

// src/command/open.rs（保留，CLI 入口）
pub fn handle_open(args: &[String], config: &YamlConfig) {
    match open_alias_core(&args[0], &args[1..], config) {
        Ok(msg) => info!("{}", msg),
        Err(e) => error!("{}", e),
    }
}
```

#### Step 3: Tauri Command 封装

```rust
// src-tauri/src/commands/alias.rs
use j_cli::core;

#[tauri::command]
pub fn open_alias(alias: String, args: Vec<String>) -> Result<String, String> {
    core::open_alias(&alias, &args)
}

#[tauri::command]
pub fn search_aliases(query: String) -> Vec<SearchResult> {
    let config = j_cli::YamlConfig::load();
    core::search_aliases(&query, &config)
}
```

### 复用度评估

```
┌─────────────────────────────────────────────────────────────┐
│                    j-cli 代码复用度                          │
├─────────────────────────────────────────────────────────────┤
│ █████████████████████████████████████░░░░░  85% 可复用      │
├─────────────────────────────────────────────────────────────┤
│ ✅ config/        100% 直接复用                              │
│ ✅ constants.rs   100% 直接复用                              │
│ ✅ util/          100% 直接复用                              │
│ ✅ command/open   90%  抽取核心逻辑                          │
│ ✅ command/alias  90%  抽取核心逻辑                          │
│ ✅ command/report 80%  核心写入逻辑复用                       │
│ ✅ command/script 85%  执行逻辑复用                          │
│ ⚠️ tui/           0%   GUI 不需要                            │
│ ⚠️ interactive/   0%   GUI 不需要                            │
└─────────────────────────────────────────────────────────────┘
```

**结论**: 约 **85%** 的核心代码可直接或经轻量封装后复用于 GUI 版本。

---

## 三、项目文件结构规划

```
j/
├── src/                          # 现有 j-cli Rust 代码（保留）
│   ├── lib.rs                   # 新增：库入口，暴露公共 API
│   ├── main.rs                  # CLI 入口（保留）
│   ├── cli.rs
│   ├── core/                    # 新增：GUI 友好的核心接口
│   │   ├── mod.rs
│   │   ├── alias.rs             # 别名操作核心逻辑
│   │   ├── open.rs              # 打开应用核心逻辑
│   │   ├── report.rs            # 日报核心逻辑
│   │   └── search.rs            # 搜索核心逻辑
│   ├── command/                 # 现有命令实现（逐步重构调用 core）
│   ├── config/
│   ├── constants.rs
│   ├── interactive/
│   ├── tui/
│   └── util/
│
├── src-tauri/                    # Tauri 后端（新增）
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/
│   │   ├── icon.icns
│   │   ├── icon.png
│   │   └── tray.png
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── app/
│   │   │   ├── mod.rs
│   │   │   ├── setup.rs
│   │   │   └── tray.rs
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── alias.rs
│   │   │   ├── search.rs
│   │   │   ├── report.rs
│   │   │   ├── todo.rs
│   │   │   ├── launcher.rs
│   │   │   └── system.rs
│   │   ├── hotkey/
│   │   │   ├── mod.rs
│   │   │   └── manager.rs
│   │   └── window/
│   │       ├── mod.rs
│   │       └── spotlight.rs
│   └── build.rs
│
├── src-ui/                       # 前端代码（新增）
│   ├── index.html
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── postcss.config.js         # PostCSS 配置（Tailwind v4 需要）
│   ├── src/
│   │   ├── main.tsx
│   │   ├── App.tsx
│   │   ├── styles/
│   │   │   └── globals.css       # Tailwind v4 配置在此文件中
│   │   ├── components/
│   │   │   ├── layout/
│   │   │   │   ├── SpotlightWindow.tsx
│   │   │   │   ├── SearchBar.tsx
│   │   │   │   └── ResultList.tsx
│   │   │   ├── features/
│   │   │   │   ├── AliasPanel.tsx
│   │   │   │   ├── ReportPanel.tsx
│   │   │   │   ├── TodoPanel.tsx
│   │   │   │   ├── ScriptPanel.tsx
│   │   │   │   └── SettingsPanel.tsx
│   │   │   └── ui/
│   │   │       ├── Button.tsx
│   │   │       ├── Input.tsx
│   │   │       ├── Dialog.tsx
│   │   │       ├── Toast.tsx
│   │   │       └── ScrollArea.tsx
│   │   ├── hooks/
│   │   │   ├── useSearch.ts
│   │   │   ├── useHotkey.ts
│   │   │   ├── useAliases.ts
│   │   │   └── useTauri.ts
│   │   ├── stores/
│   │   │   ├── searchStore.ts
│   │   │   ├── aliasStore.ts
│   │   │   └── settingsStore.ts
│   │   ├── services/
│   │   │   └── tauri.ts
│   │   ├── types/
│   │   │   └── index.ts
│   │   └── utils/
│   │       └── index.ts
│   └── public/
│
├── Cargo.toml                    # Workspace 配置
├── Cargo.lock
└── README.md
```

---

## 四、核心模块设计

### 1. 窗口系统（Spotlight 风格）

```
┌─────────────────────────────────────────────────────────────┐
│                      macOS Desktop                          │
│                                                             │
│         ┌─────────────────────────────────────┐            │
│         │  🔍 搜索别名、命令、应用...          │            │
│         └─────────────────────────────────────┘            │
│         ┌─────────────────────────────────────┐            │
│         │ 📁 chrome      → 打开浏览器         │            │
│         │ 📝 report      → 写入日报           │            │
│         │ ✅ todo        → 待办事项           │            │
│         │ 🚀 script      → 执行脚本           │            │
│         └─────────────────────────────────────┘            │
│                                                             │
│                                       [🚀 托盘图标]          │
└─────────────────────────────────────────────────────────────┘
```

**窗口特性**：
- 居中悬浮显示
- 毛玻璃背景效果
- 动画进入/退出
- ESC 关闭，点击外部关闭
- 无任务栏图标（隐藏 dock）

### 2. 系统托盘整合

托盘菜单结构：
```
┌─────────────────────┐
│ 🔍 打开搜索         │
├─────────────────────┤
│ 📝 快速写日报       │
│ ✅ 查看待办         │
├─────────────────────┤
│ 🚀 快捷别名         │
│    └─ chrome        │
│    └─ vscode        │
│    └─ ...           │
├─────────────────────┤
│ ⚙️ 设置            │
│ ❌ 退出            │
└─────────────────────┘
```

---

## 五、分阶段实现计划

### Phase 1: 基础框架 (Week 1)
- [ ] 初始化 Tauri 项目结构
- [ ] 配置 Cargo workspace
- [ ] 创建 `src/lib.rs` 和 `src/core/` 模块
- [ ] 实现基础 Spotlight 窗口
- [ ] 全局快捷键 `Cmd + J` 注册
- [ ] 系统托盘基础功能

### Phase 2: 核心功能 (Week 2)
- [ ] 重构 `command/open.rs` 抽取核心逻辑
- [ ] 重构 `command/alias.rs` 抽取核心逻辑
- [ ] 别名搜索与打开（GUI）
- [ ] 搜索结果模糊匹配
- [ ] 日报快速写入

### Phase 3: 扩展功能 (Week 3)
- [ ] 脚本执行
- [ ] 设置面板
- [ ] 托盘菜单完善
- [ ] 开机自启动

### Phase 4: 优化打磨 (Week 4)
- [ ] 动画效果
- [ ] 性能优化
- [ ] 错误处理
- [ ] 打包发布

---

## 六、关键配置文件

### 1. Cargo.toml (Workspace)

```toml
[workspace]
members = ["src-tauri"]
resolver = "2"

[workspace.package]
version = "0.1.0"
authors = ["lingojack"]
edition = "2021"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
```

### 2. package.json (前端依赖)

```json
{
  "name": "j-cli-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "zustand": "^5.0.0",
    "@tauri-apps/api": "^2.0.0"
  },
  "devDependencies": {
    "@tailwindcss/vite": "^4.0.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "tailwindcss": "^4.0.0",
    "typescript": "^5.7.0",
    "vite": "^6.0.0"
  }
}
```

### 3. globals.css (Tailwind v4 配置)

```css
/* Tailwind v4 配置内置于 CSS 文件 */
@import "tailwindcss";

/* 自定义主题配置 */
@theme {
  --color-primary: #007AFF;
  --color-secondary: #5856D6;
  --color-background: rgba(30, 30, 30, 0.85);
  --color-surface: rgba(45, 45, 45, 0.9);
  --color-border: rgba(255, 255, 255, 0.1);
  --color-text: #FFFFFF;
  --color-text-secondary: rgba(255, 255, 255, 0.6);
  
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 14px;
  
  --blur-glass: blur(20px);
}

/* 毛玻璃效果 */
.glass {
  background: var(--color-background);
  backdrop-filter: var(--blur-glass);
  -webkit-backdrop-filter: var(--blur-glass);
}

/* macOS 风格滚动条 */
::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 3px;
}
```

### 4. vite.config.ts

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
```

### 5. tauri.conf.json (核心配置)

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "j-cli-gui",
  "version": "0.1.0",
  "identifier": "com.lingojack.jcli",
  "build": {
    "frontendDist": "../src-ui/dist",
    "devUrl": "http://localhost:5173"
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      {
        "label": "spotlight",
        "title": "j-cli",
        "width": 600,
        "height": 400,
        "center": true,
        "resizable": false,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "visible": false
      }
    ],
    "trayIcon": {
      "iconPath": "icons/tray.png",
      "iconAsTemplate": true
    },
    "macOSPrivateApi": true
  },
  "plugins": {
    "global-shortcut": {
      "shortcuts": ["CommandOrControl+J"]
    }
  }
}
```

---

## 七、技术风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 全局快捷键冲突 | 中 | `Cmd + J` 冲突较少，提供可配置 |
| 毛玻璃性能 | 低 | 使用系统原生效果 |
| 权限问题 | 中 | 正确配置 entitlements |
| 代码重构工作量 | 中 | 渐进式重构，保持 CLI 稳定 |

---

## 八、确认事项（已确认）

- **全局快捷键**: `Cmd + J`
- **代码复用**: 约 85% 核心代码可复用
- **Breaking Change**: **零风险**，采用"只增不改"策略
  - 新增 `src/core/` 模块，不修改现有 `command/` 模块
  - 现有 CLI 功能保持 100% 兼容
  - 仅将部分内部函数可见性改为 `pub(crate)`
