# 解决安装脚本版本号同步问题

## 问题分析

### 1. install.sh / install.ps1 的问题
- 脚本中硬编码了示例版本号 `v1.0.0`（当前实际版本是 `v12.10.64`）
- 网络获取失败时的错误提示显示过时的版本号
- 每次 `make publish` 发布新版本后，脚本中的示例版本号没有同步更新

### 2. update.rs 的改进空间
- `VERSION` 常量来自编译时 `Cargo.toml` 的 `CARGO_PKG_VERSION`，每次 publish 都是先 bump-version 再编译，所以 `VERSION` 就是当前最新版本号
- 当 `get_latest_version_fallback()` 所有网络方法都失败时，可以用编译时嵌入的 `VERSION` 作为回退版本号去下载
- 这样用户在网络不好（比如 GitHub API 403 rate limit）时，仍然可以成功更新

## 解决方案

### 方案设计

采用**集中管理 + 自动同步 + 编译时回退**策略：

1. 在 `install.sh` 和 `install.ps1` 顶部定义 `DEFAULT_VERSION` 常量
2. 在 Makefile 的 `bump-version` / `set-version` 目标中，自动同步更新脚本中的版本号
3. `update.rs` 中 `get_latest_version_fallback()` 失败时，用编译时嵌入的 `VERSION` 作为回退

### 需要修改的文件

| 文件 | 修改内容 |
|------|---------|
| `install.sh` | 顶部添加 `DEFAULT_VERSION="v12.10.64"`，替换所有硬编码的 `v1.0.0` |
| `install.ps1` | 顶部添加 `$DefaultVersion = "v12.10.64"`，替换所有硬编码的 `v1.0.0` |
| `Makefile` | `bump-version` 和 `set-version` 目标中增加同步更新脚本版本号的逻辑 |
| `update.rs` | `get_latest_version_fallback()` 返回 `None` 时，回退到编译时 `VERSION` |

### 详细修改方案

#### 1. install.sh 修改

- 顶部新增：`DEFAULT_VERSION="v12.10.64"`
- 替换所有硬编码的 `v1.0.0` 为 `$DEFAULT_VERSION`
- 网络获取失败时的错误提示使用 `$DEFAULT_VERSION`

**需要替换的位置（3处）**：
- 注释中的示例用法
- 错误提示中的示例版本
- 帮助信息中的示例版本

#### 2. install.ps1 修改

- 顶部新增：`$DefaultVersion = "v12.10.64"`
- 替换所有硬编码的 `v1.0.0` 为 `$DefaultVersion`
- 网络获取失败时的错误提示使用 `$DefaultVersion`

**需要替换的位置（约6处）**：
- 注释中的示例用法（2处）
- 错误提示中的示例版本（1处）
- 帮助信息中的示例版本（3处）

#### 3. Makefile 修改

在 `bump-version` 目标中增加：
```bash
# 同步更新安装脚本中的 DEFAULT_VERSION
sed -i '' "s/DEFAULT_VERSION=\"v[^\"]*\"/DEFAULT_VERSION=\"v$$new_version\"/" install.sh
sed -i '' 's/\$DefaultVersion = "v[^"]*"/$DefaultVersion = "v'"$$new_version"'"/' install.ps1
```

同样在 `set-version` 目标中增加相同的逻辑。

#### 4. update.rs 修改

核心思路：`get_latest_version_fallback()` 失败时，用编译时嵌入的 `VERSION` 作为回退。

修改 `perform_update_fallback()` 函数（第 605-626 行）：

```rust
fn perform_update_fallback(target: &str, interactive: bool) {
    let version = get_latest_version_fallback();

    // 网络获取失败时，回退到编译时嵌入的版本号
    let version = match version {
        Some(v) => v,
        None => {
            let fallback = format!("v{}", VERSION);
            println!(
                "{}",
                "无法通过网络获取最新版本号，将尝试使用编译时版本号进行更新".yellow()
            );
            println!(
                "回退版本: {} (当前安装: {})",
                fallback.cyan(),
                VERSION.cyan()
            );
            fallback
        }
    };

    // 如果回退版本与当前版本相同，说明已经是最新版本
    let ver_without_v = version.trim_start_matches('v');
    if ver_without_v == VERSION {
        println!("{}", "当前已是最新版本，无需更新".green());
        return;
    }

    // ... 后续下载逻辑不变
}
```

这样当 GitHub API 403 rate limit 时，用户仍然可以成功更新到编译时嵌入的版本（通常就是最新版本或接近最新版本）。

## 实施步骤

1. 修改 `install.sh`：添加 `DEFAULT_VERSION` 常量，替换所有硬编码的 `v1.0.0`
2. 修改 `install.ps1`：添加 `$DefaultVersion` 变量，替换所有硬编码的 `v1.0.0`
3. 修改 `Makefile` 的 `bump-version` 和 `set-version` 目标：增加同步更新脚本版本号
4. 修改 `update.rs` 的 `perform_update_fallback()`：网络失败时回退到编译时 `VERSION`
5. 验证 `cargo clippy` 和 `cargo fmt` 通过

## 预期效果

- 每次 `make publish` 发布新版本时，安装脚本中的示例版本号自动同步
- 用户在网络获取失败时，安装脚本显示真实的可用版本号
- `update.rs` 在 GitHub API 403 时，使用编译时嵌入的版本号作为回退，仍然可以完成更新
- 整体减少因网络问题导致的安装/更新失败