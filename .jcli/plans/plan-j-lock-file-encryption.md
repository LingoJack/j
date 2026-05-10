# `j lock` - 文件加密/解密命令实现计划

## 命令接口设计

```
j lock <password> <file_or_dir>        # 加密（默认当前目录）
j lock <password> .                    # 加密当前目录所有文件
j lock <password> secret.txt           # 加密单个文件
j lock <password> ~/docs              # 加密目录下所有文件

j unlock <password> <file_or_dir>      # 解密
j unlock <password> .                  # 解密当前目录所有 .lock 文件
j unlock <password> secret.txt.lock    # 解密单个文件
```

- **密码不存储**：密码仅用于派生密钥，用完即丢弃，不写入任何文件或配置
- **子命令**：`lock`（加密）/ `unlock`（解密）
- **默认路径**：不传路径时默认为当前目录 `.`
- **目录支持**：传入目录时递归处理其中所有文件（跳过 `.lock` 文件和隐藏目录）
- **文件后缀**：加密后生成 `<原文件名>.lock`，解密时去掉 `.lock` 后缀

## 加密方案

- **密钥派生**：Password + 随机 32 字节 Salt → HKDF-SHA256 → 256-bit AES Key
- **加密算法**：AES-256-GCM（认证加密，防篡改）
- **文件格式**：`MAGIC(4) + VERSION(1) + SALT(32) + NONCE(12) + CIPHERTEXT+TAG`
  - MAGIC: `JLCK`（标识 j-cli lock 文件）
  - VERSION: `0x01`（预留版本升级）
  - SALT: 32 字节随机盐
  - NONCE: 12 字节随机 nonce
  - CIPHERTEXT+TAG: AES-GCM 密文 + 16 字节认证标签
- **密码输入**：通过命令行参数直接传入（交互模式同样）

## 需修改的文件（6 个）

| # | 文件 | 修改内容 |
|---|------|----------|
| 1 | `src/cli.rs` | 新增 `Lock` 和 `Unlock` 子命令变体 |
| 2 | `src/command.rs` | 新增 `pub mod lock;` |
| 3 | `src/command/handler.rs` | 在 `command_handlers!` 宏中注册 `LockCmd` / `UnlockCmd`，在 `into_handler()` 中添加 match 分支 |
| 4 | `src/command/lock.rs` | **新建**：加密/解密核心逻辑 |
| 5 | `src/constants.rs` | 添加 `cmd::LOCK` 和 `cmd::UNLOCK` 常量 + 加入 `all_keywords()` |
| 6 | `src/interactive/parser.rs` | 添加 `lock` / `unlock` 解析分支 |

## 核心实现 (`src/command/lock.rs`)

```rust
// 主要函数：
pub fn handle_lock(password: &str, target: &str)    // 加密入口
pub fn handle_unlock(password: &str, target: &str)  // 解密入口

// 内部函数：
fn encrypt_file(password: &str, path: &Path) -> Result<()>
fn decrypt_file(password: &str, path: &Path) -> Result<()>
fn derive_key(password: &str, salt: &[u8; 32]) -> [u8; 32]
fn collect_files(target: &str, extension_filter: Option<&str>) -> Vec<PathBuf>
```

### 目录递归处理规则
- 加密时：收集所有普通文件，跳过已有 `.lock` 后缀的文件
- 解密时：收集所有 `.lock` 后缀的文件
- 跳过隐藏目录（`.git`、`.jcli` 等）
- 跳过符号链接

### 错误处理
- 文件不存在 → `error!("文件不存在: {}", path)`
- 解密失败（密码错误）→ `error!("解密失败，密码错误或文件已损坏: {}", path)`
- 权限不足 → `error!("无权限访问: {}", path)`
- 部分文件失败时继续处理其余文件，最后汇总报告

## 依赖

无需新增依赖。已有：
- `aes-gcm` 0.10 — AES-256-GCM 加密
- `hkdf` 0.13 — 密钥派生
- `sha2` 0.11 — SHA-256 哈希
- `rand` 0.8 — 随机数生成

## 使用示例

```bash
# CLI 模式
j lock mypassword secret.txt          # → 生成 secret.txt.lock
j unlock mypassword secret.txt.lock   # → 还原 secret.txt
j lock mypassword ~/docs              # → 递归加密 ~/docs 下所有文件
j unlock mypassword ~/docs            # → 递归解密 ~/docs 下所有 .lock 文件

# 交互模式
j > lock mypassword secret.txt
j > unlock mypassword .
```
