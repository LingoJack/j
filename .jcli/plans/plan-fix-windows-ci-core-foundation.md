# Fix: Windows CI 编译失败 - core-foundation 在 Windows 上无法编译

## 问题分析

Windows CI (`build-windows-x64` / `build-windows-arm64`) 编译失败，根因是 `j-agent/Cargo.toml` 中 `core-graphics` 和 `nix` 被声明为**无条件依赖**（第 67-68 行），而非平台条件依赖。

虽然代码中已经用 `#[cfg(target_os = "macos")]` / `#[cfg(unix)]` 保护了对应模块的使用，但 Cargo 会**无条件下载和编译**所有声明的依赖及其传递依赖。`core-graphics` -> `core-foundation` 0.10.1 引用了 `std::os::unix` 和 `libc::PATH_MAX`，在 Windows 上编译失败。

主 `Cargo.toml` 已正确使用 `[target.'cfg(target_os = "macos")'.dependencies]` 和 `[target.'cfg(unix)'.dependencies]`，只有 `j-agent/Cargo.toml` 缺失这个约束。

## 修改方案

### 文件: `j-agent/Cargo.toml`

1. **删除** `[dependencies]` 中的两行无条件依赖：
   ```toml
   core-graphics = "0.25"
   nix = { version = "0.31", features = ["process", "signal"] }
   ```

2. **添加** 平台条件依赖段：
   ```toml
   # Unix-only
   [target.'cfg(unix)'.dependencies]
   nix = { version = "0.31", features = ["process", "signal"] }

   # macOS-only
   [target.'cfg(target_os = "macos")'.dependencies]
   core-graphics = "0.25"
   ```

这样：
- `nix` 只在 Unix（macOS + Linux）上被编译
- `core-graphics` 只在 macOS 上被编译
- `core-foundation` 作为 `core-graphics` 的传递依赖也只在 macOS 上被编译
- Windows CI 不再尝试编译这些 crate

## 验证

修改后在 macOS 上运行 `cargo check` 确认无回归。
