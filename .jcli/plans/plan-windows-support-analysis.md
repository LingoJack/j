# j-cli Windows 支持改动分析

## 概览

当前项目（j-cli v12.10.3）高度依赖 macOS/Unix 平台特性，支持 Windows 需要在 **依赖项、平台特定代码、构建流水线、安装脚本** 四个层面进行改动。以下按优先级（Critical / Medium / Low）分类。

---

## 一、Critical（必须改动，否则无法编译）

### 1.1 Cargo.toml 依赖条件化

当前以下依赖 **无平台限制**，在 Windows 上会编译失败：

| 依赖 | 用途 | Windows 方案 |
|------|------|-------------|
| `core-graphics = "0.25"` | macOS 截图/鼠标/键盘事件模拟 | 改为 `target = "cfg(target_os = \"macos\")"` |
| `core-foundation = "0.10"` | macOS Core Foundation 类型 | 同上 |
| `nix = "0.31"` | Unix 信号/进程管理 (SIGKILL, execv) | 改为 `target = "cfg(unix)"` |
| `libc = "0.2"` | Unix C FFI | 改为 `target = "cfg(unix)"` |

**改动方式**：使用 Cargo 的 `[target.'cfg(...)'.dependencies]` 或平台 target specification：

```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.25"
core-foundation = "0.10"

[target.'cfg(unix)'.dependencies]
nix = { version = "0.31", features = ["process", "signal"] }
libc = "0.2"
```

### 1.2 Shell 执行硬编码 `bash`（2处）

**文件**: `src/command/chat/tools/shell.rs`
- 第 177 行: `std::process::Command::new("bash")`
- 第 453 行: `std::process::Command::new("bash")`

**改动方式**：抽象一个 `get_shell()` 工具函数：
```rust
fn get_shell_command() -> std::process::Command {
    if cfg!(windows) {
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/C");
        cmd
    } else {
        let mut cmd = std::process::Command::new("bash");
        cmd.arg("-c");
        cmd
    }
}
```

同时注意 bash 的 `-c` 参数和 cmd 的 `/C` 参数语义差异（引号处理、管道、多命令分隔符等）。

### 1.3 `nix::unistd::execv` 进程替换（update.rs）

**文件**: `src/command/update.rs` 第 871 行

```rust
let err = nix::unistd::execv(&exe_cstr, &[&exe_cstr]);
```

这是 macOS/Linux 的自更新重启机制。Windows 不支持 `execv`。

**改动方式**：
```rust
#[cfg(unix)]
{
    let err = nix::unistd::execv(&exe_cstr, &[&exe_cstr]);
    println!("重启失败: {:?}", err);
}

#[cfg(windows)]
{
    // Windows: 启动新进程后退出当前进程
    std::process::Command::new(&exe).spawn().ok();
    std::process::exit(0);
}
```

### 1.4 `nix::sys::signal::kill` 发送 SIGKILL（hook executor）

**文件**: `src/command/chat/infra/hook/executor.rs` 第 337-340 行

```rust
let _ = nix::sys::signal::kill(
    nix::unistd::Pid::from_raw(pid as i32),
    nix::sys::signal::Signal::SIGKILL,
);
```

**改动方式**：
```rust
#[cfg(unix)]
{
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid as i32),
        nix::sys::signal::Signal::SIGKILL,
    );
}
#[cfg(windows)]
{
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .status();
}
```

### 1.5 `computer_use` 模块整体条件化（Windows 不支持，直接禁用）

**文件**: `src/command/chat/tools/computer_use/` 下所有文件

整个 `computer_use` 模块依赖 macOS 的 `CoreGraphics` 框架（截图、鼠标、键盘事件模拟）和辅助二进制 `j-ax`（Accessibility API）。**Windows 不需要支持此功能**，直接条件编译排除即可。

**改动方式**：
- 在 `src/command/chat/tools/` 的 `mod.rs` 中条件化：
  ```rust
  #[cfg(target_os = "macos")]
  pub mod computer_use;
  ```
- 在 `computer_use/tool.rs` 的 tool 注册/发现逻辑中，确保该 tool 仅在 macOS 上注册
- 将 `core-graphics` / `core-foundation` 依赖移到 macOS 专属的 `[target.'cfg(target_os = "macos")'.dependencies]`

---

## 二、Medium（功能性改动，影响核心体验）

### 2.1 `self_update` 配置

**文件**: `src/command/update.rs`

- 第 98-106 行：`ReleaseList::configure()` 设置的 `bin_name` 在 Windows 上需要 `.exe` 后缀
- 第 251-280 行：`Update::configure()` 同理
- `archive-tar` + `compression-flate2` 特性用于 `.tar.gz`，Windows 发布包通常用 `.zip`

**改动方式**：
```toml
[target.'cfg(unix)'.dependencies]
self_update = { version = "0.42", default-features = false, features = ["archive-tar", "compression-flate2", "rustls"] }

[target.'cfg(windows)'.dependencies]
self_update = { version = "0.42", default-features = false, features = ["archive-zip", "rustls"] }
```

同时在代码中处理 `bin_name` 的平台差异：
```rust
let bin_name = if cfg!(windows) { "j.exe" } else { "j" };
```

### 2.2 文件权限设置（5处）

以下位置使用 `std::os::unix::fs::PermissionsExt`，已有 `#[cfg(unix)]` 保护，但 Windows 上缺少替代方案：

| 文件 | 行号 | 功能 |
|------|------|------|
| `src/command/update.rs` | 674 | 更新后设置可执行权限 |
| `src/command/update.rs` | 819 | curl 更新后设置可执行权限 |
| `src/command/script.rs` | 143 | 创建脚本后设置执行权限 |
| `src/command/open.rs` | 408 | 检查文件可执行权限 |
| `src/command/assets/install.rs` | 141 | 安装资源文件设置 755 |

**改动方式**：大部分已有 `#[cfg(unix)]` 保护，Windows 上只需：
- 确认 Windows 上 `.exe` 后缀判断逻辑已实现（`open.rs` 第 415-421 行已有）
- `update.rs` 中的权限设置在 Windows 上无需操作（NTFS 无 Unix 权限位）

### 2.3 macOS 签名与隔离属性

**文件**: `src/command/update.rs` 第 1-15 行及多处

- `codesign` 签名（第 700+ 行）
- `xattr -d com.apple.quarantine` 移除隔离标记（第 800+ 行）
- Apple Silicon 上未签名二进制被 SIGKILL 的说明

**改动方式**：用 `#[cfg(target_os = "macos")]` 条件编译包裹整个签名/隔离逻辑。

### 2.4 `shell_safety.rs` - Shell 命令安全检查

**文件**: `src/util/shell_safety.rs`

安全规则基于 bash/Unix 命令语法（`rm -rf`, `chmod`, `chown` 等）。Windows 下：
- 危险命令不同（`del /s /q`, `format`, `rd /s /q` 等）
- PowerShell 有不同的危险模式

**改动方式**：扩展 `DANGEROUS_PATTERNS` 列表，增加 Windows/PowerShell 危险命令。

### 2.5 常量定义

**文件**: `src/constants.rs` 第 404 行

```rust
pub const BASH_PATH: &str = "/bin/bash";
```

**改动方式**：条件编译或运行时判断。

### 2.6 脚本 shebang

**文件**: `src/command/script.rs` 第 73 行

```rust
"#!/bin/bash".to_string(),
```

Windows 上不支持 shebang。

**改动方式**：
```rust
let shebang = if cfg!(windows) { "" } else { "#!/bin/bash" };
```

### 2.7 辅助二进制 helpers/

| 文件 | 用途 | Windows 方案 |
|------|------|-------------|
| `helpers/indicator.swift` | macOS 菜单栏指示灯（NSStatusItem） | 不适用，功能取消 |
| `helpers/ax.swift` | macOS Accessibility API 桥接 | 不适用，功能取消 |

这两个 Swift 辅助工具是 macOS 专属的，Windows 上无等价物。需要：
- 条件化安装逻辑（不安装这些文件）
- 条件化调用这些工具的代码路径

---

## 三、Low（体验优化，非阻塞）

### 3.1 安装脚本 `install.sh`

当前仅有 bash 安装脚本，Windows 需要：
- `install.ps1` (PowerShell) 安装脚本
- 或 winget/scoop/chocolatey 包定义

### 3.2 CI/CD（`.github/workflows/release.yml`）

当前仅构建 macOS ARM64。需要添加：
- `windows-latest` runner 的构建 job
- Windows 产物打包（`.zip` 而非 `.tar.gz`）
- 考虑添加 Windows 代码签名

### 3.3 路径处理

- `~` 展开：`dirs` crate 已处理，无问题
- 硬编码的 `/tmp` 等路径需检查（应使用 `std::env::temp_dir()`）
- 路径分隔符 `\` vs `/`：Rust 的 `std::path` 已抽象，大部分无问题

### 3.4 TUI 兼容性

- `crossterm` 0.28 已支持 Windows（启用 `windows` feature 默认开启）
- `ratatui` 0.29 跨平台无问题
- `rustyline` Windows 支持良好
- `arboard`（剪贴板）跨平台支持

### 3.5 `open` 命令

**文件**: `src/command/open.rs`

已有 Windows 分支（第 471-474 行）：
```rust
Command::new(shell::WINDOWS_CMD)
    .args([shell::WINDOWS_CMD_FLAG, "start", "", path])
```
这部分已处理，无需额外改动。

---

## 四、改动量估算

| 优先级 | 改动项 | 预估工作量 |
|--------|--------|-----------|
| Critical | Cargo.toml 依赖条件化 | 0.5 天 |
| Critical | Shell 执行 bash→跨平台 | 0.5 天 |
| Critical | update.rs execv 替换 | 0.5 天 |
| Critical | hook executor kill 信号 | 0.5 天 |
| Critical | computer_use 模块条件化（禁用） | 0.5 天 |
| Medium | self_update zip 支持 | 0.5 天 |
| Medium | 签名/隔离逻辑条件化 | 0.5 天 |
| Medium | shell_safety 扩展 | 0.5 天 |
| Medium | 脚本 shebang/权限处理 | 0.5 天 |
| Medium | helpers 条件化 | 0.5 天 |
| Low | install.ps1 安装脚本 | 1 天 |
| Low | CI Windows 构建 | 1 天 |
| Low | 端到端测试 | 2 天 |

**总计约 7-8 个工作日**。

---

## 五、推荐实施路径

1. **Phase 1 - 编译通过**: 改 Cargo.toml + 条件化 computer_use → Windows 上能 `cargo build`
2. **Phase 2 - 核心功能**: Shell 执行 + update 机制 + hook 执行 → 核心功能可用
3. **Phase 3 - 完整体验**: 安装脚本 + CI + shell_safety → 可发布
