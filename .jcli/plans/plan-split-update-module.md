# 拆分 `src/command/update.rs` 方案

## 现状

`src/command/update.rs` 是一个 1154 行的单文件，包含以下功能区域：

1. **平台工具函数** (L1-41): `fix_codesign_and_quarantine` - macOS 代码签名修复
2. **GitHub 认证** (L43-66): `get_github_auth_token` - 获取 GitHub token
3. **入口分发** (L68-75): `handle_update` - 根据 INSTALL_SOURCE 分发
4. **GitHub 更新** (L77-87): `handle_github_update`
5. **版本检查** (L89-138): `check_for_update`
6. **权限检测与提升** (L140-274): `perform_update` - 含 macOS osascript/Windows/Linux 权限逻辑
7. **self_update 核心更新** (L277-355): `perform_update_internal` - 使用 self_update crate
8. **Feature 选择 UI** (L357-525): `OPTIONAL_FEATURES`, `select_features`, `draw_feature_menu` - TUI 交互
9. **Cargo 更新** (L527-594): `handle_cargo_update` - cargo install 更新
10. **未知来源** (L597-600): `show_unknown_source_hint`
11. **备用更新方案** (L602-894): `perform_update_fallback` - curl/PowerShell 下载更新
12. **备用版本获取** (L896-1012): `get_latest_version_fallback` - 从 GitHub API/重定向获取版本
13. **j-indicator 安装** (L1014-1110): `install_indicator_from_release` - macOS indicator 同步安装
14. **进程重启** (L1112-1154): `restart_self` - execv 替换进程

## 拆分方案

遵循项目约定（`name.rs` + `name/` 子目录，弃用 mod.rs），拆分为：

```
src/command/update.rs          → src/command/update.rs       (入口 + 公共 API)
                                  src/command/update/
```

### 文件划分

| 文件 | 职责 | 包含函数 |
|------|------|----------|
| `update.rs` | 入口分发 + 公共 API | `handle_update`, `handle_github_update`, `show_unknown_source_hint` |
| `update/codesign.rs` | 平台代码签名/权限工具 | `fix_codesign_and_quarantine` |
| `update/github_auth.rs` | GitHub 认证 | `get_github_auth_token` |
| `update/check.rs` | 版本检查 | `check_for_update` |
| `update/permission.rs` | 权限检测与提升 | `check_write_permission`, `elevate_and_update` (从 `perform_update` 中拆出权限相关逻辑) |
| `update/github_update.rs` | self_update 核心更新 | `perform_update`, `perform_update_internal` |
| `update/cargo_update.rs` | Cargo 更新 | `handle_cargo_update` |
| `update/fallback.rs` | 备用更新方案 | `perform_update_fallback`, `get_latest_version_fallback` |
| `update/indicator.rs` | j-indicator 安装 | `install_indicator_from_release` |
| `update/restart.rs` | 进程重启 | `restart_self` |
| `update/feature_select.rs` | Feature 选择 UI | `OPTIONAL_FEATURES`, `menu_total_lines`, `select_features`, `draw_feature_menu` |

### 模块声明

`update.rs` 中声明子模块并重新导出必要的项：

```rust
mod codesign;
mod github_auth;
mod check;
mod permission;
mod github_update;
mod cargo_update;
mod fallback;
mod indicator;
mod restart;
mod feature_select;
```

### 外部依赖不变

`src/command.rs` 中的 `pub mod update;` 无需修改，外部调用 `command::update::handle_update` 不变。

### 实施步骤

1. 创建 `src/command/update/` 目录
2. 按上述划分将函数移入对应子文件
3. 修改 `update.rs` 为模块入口，声明子模块 + `pub(crate)` 重新导出 `handle_update`
4. 处理跨文件引用（如 `github_auth::get_github_auth_token` 在多个文件中使用）
5. 运行 `cargo check` / `cargo clippy` 验证编译通过
