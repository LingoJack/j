# Plan: Rust Crates 依赖升级策略

## 项目概况

这是一个 Rust TUI 应用，当前使用 Rust 1.93.1, edition 2024。项目有一个本地 patch (`tui-textarea`)。

## 版本对比

| Crate | 当前版本 | 最新版本 | 升级类型 | 风险等级 |
|-------|---------|---------|---------|---------|
| **核心依赖** |
| tokio | 1.47.0 | 1.50.0 | Minor | 低 |
| serde | 1.0.219 | 1.0.228 | Patch | 低 |
| serde_json | 1.0.143 | 1.0.149 | Patch | 低 |
| **CLI/终端** |
| clap | 4.5.47 | 4.5.47 | 已最新 | - |
| crossterm | 0.28.1 | 0.29.0 | Minor | 中 |
| ratatui | 0.29.0 | 0.30.0 | Minor | 中 |
| tui-textarea | 0.7.0 (patch) | 0.7.0 | 本地 patch | 需检查兼容性 |
| ratatui-image | 9.0.0 | 10.0.6 | Major | 高 |
| **网络/HTTP** |
| reqwest | 0.12.23 | 0.13.2 | Minor | 中 |
| async-openai | 0.29.0 | 0.34.0 | Minor | 中 |
| url | 2.5.4 | 2.5.8 | Patch | 低 |
| **工具库** |
| anyhow | 1.0.100 | 1.0.100 | 已最新 | - |
| thiserror | 2.0.16 | 2.0.16 | 已最新 | - |
| tracing | 0.1.41 | 0.1.41 | 已最新 | - |
| tracing-subscriber | 0.3.20 | 0.3.20 | 已最新 | - |
| chrono | 0.4.41 | 0.4.41 | 已最新 | - |
| uuid | 1.18.1 | 1.18.1 | 已最新 | - |
| base64 | 0.22.1 | 0.22.1 | 已最新 | - |
| sha2 | 0.10.9 | 0.11.0 | Minor | 中 |
| regex | 1.11.2 | 1.11.2 | 已最新 | - |
| tempfile | 3.21.0 | 3.21.0 | 已最新 | - |
| futures | 0.3.31 | 0.3.31 | 已最新 | - |
| tokio-stream | 0.1.17 | 0.1.18 | Patch | 低 |
| log | 0.4.27 | 0.4.27 | 已最新 | - |
| dirs | 6.0.0 | 6.0.0 | 已最新 | - |
| directories | 6.0.0 | 6.0.0 | 已最新 | - |
| percent-encoding | 2.3.1 | 2.3.2 | Patch | 低 |
| **系统/平台** |
| nix | 0.30.1 | 0.30.1 | 已最新 | - |
| arboard | 3.6.1 | 3.6.1 | 已最新 | - |
| self_update | 0.42.0 | 0.43.1 | Minor | 中 |
| ctrlc | 3.5.0 | 3.5.2 | Patch | 低 |
| **已弃用/特殊** |
| serde_yaml | 0.9.34 | 0.9.34+deprecated | 已弃用 | 建议迁移 |
| spinners | 4.2.0 | 4.2.0 | 已最新 | - |
| jsonwebtoken | 10.2.0 | 10.3.0 | Minor | 中 |
| openai-api-types | 0.5.0 | ? | 需确认 | 待查 |

## 风险分析

### 高风险升级
1. **ratatui-image 9.0.0 → 10.0.6** (Major 版本跳变)
   - 可能有不兼容的 API 变更
   - 需要仔细检查 CHANGELOG

### 中风险升级
1. **crossterm 0.28.1 → 0.29.0** - 终端控制库，可能影响事件处理
2. **ratatui 0.29.0 → 0.30.0** - TUI 框架，可能有 API 变更
3. **reqwest 0.12.23 → 0.13.2** - HTTP 客户端，可能有配置变更
4. **async-openai 0.29.0 → 0.34.0** - OpenAI API 客户端，多个版本跨越
5. **sha2 0.10.9 → 0.11.0** - 哈希库，可能有 API 变更
6. **self_update 0.42.0 → 0.43.1** - 自更新功能
7. **jsonwebtoken 10.2.0 → 10.3.0** - JWT 库

### 需要特别处理
1. **tui-textarea** - 本地 patch 需要检查是否与新版本 ratatui 兼容
2. **serde_yaml** - 已标记 deprecated，建议迁移到其他 YAML 库

## 升级策略

### 阶段一：安全升级（低风险）
- tokio, serde, serde_json, url, tokio-stream, percent-encoding, ctrlc
- 这些都是 patch 或 minor 升级，兼容性好

### 阶段二：中风险升级
- crossterm, ratatui, reqwest, async-openai, sha2, self_update, jsonwebtoken
- 升级后需要运行测试验证

### 阶段三：高风险升级
- ratatui-image (Major 版本)
- 需要单独处理，可能需要修改代码

### 阶段四：清理
- 处理 serde_yaml deprecated 问题
- 验证本地 patch 兼容性

## 执行步骤

1. **备份当前状态** - 创建 git stash 或 commit
2. **更新 Cargo.toml** - 修改版本号
3. **运行 `cargo update`** - 更新 Cargo.lock
4. **运行 `cargo check`** - 检查编译错误
5. **修复 breaking changes** - 根据错误信息修改代码
6. **运行测试** - `cargo test`
7. **构建验证** - `cargo build --release`

## 建议

1. **分批升级**：先升级低风险的，确认没问题再升级高风险的
2. **保留 patch**：tui-textarea 的本地 patch 需要保留
3. **关注 MSRV**：部分新版本可能要求更高 Rust 版本
4. **serde_yaml**：考虑迁移到 `yaml_serde` 或其他维护中的 YAML 库
