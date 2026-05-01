# Teammate 健壮性提升：派生 Agent 重试次数放宽到 8 次

## 改动

将 `derived_retry_policy` 中所有错误类型的 `max_attempts` 统一放宽到 8，同时提高退避上限：

| 错误类型 | 改前 | 改后 |
|---|---|---|
| 网络超时/断连 | 5 / cap 15s | 8 / cap 30s |
| 503/504/529 过载 | 4 / cap 15s | 8 / cap 30s |
| 500/502 服务端错误 | 3 / cap 15s | 8 / cap 30s |
| 429 rate limit | 3 / cap 30s | 8 / cap 60s |
| 异常 finish | 4 / cap 15s | 8 / cap 30s |
| 兜底过载 | 3 / cap 15s | 8 / cap 30s |

## 修改文件

仅修改 `src/command/chat/tools/derived_shared.rs` 中的 `derived_retry_policy` 函数。
