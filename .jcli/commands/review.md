---
name: code-review
description: 根据规范进行 code review
---
请根据 AGENT.md 规范检查是否有不符合规范的地方。

## 辅助工具

项目提供了自动化的合规性检查脚本，可先运行获取基础报告，再在此基础上进行深度审查：

```bash
# 运行 11 项自动化检查（格式、clippy、文件行数、函数行数、参数数量、unwrap/mod.rs/super::super::、文档注释、TUI 输出规范、unsafe SAFETY 注释等）
make check-lint

# 带 --fix 参数会自动执行 cargo fmt
bash scripts/check_lint.sh --fix
```

建议流程：先执行 `make check-lint` 拿到基础报告，再针对报告中的 WARN/FAIL 项结合 AGENT.md 软性规范（单一职责、类型设计、模式匹配偏好等）做补充审查。
