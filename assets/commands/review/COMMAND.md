---
name: review
description: "Review code changes in the current git repository. Analyzes staged or recent commits for bugs, style issues, and improvements."
---

请对当前 git 仓库的代码变更进行 Code Review。

## 步骤

1. 先用 `git diff` 或 `git log` 了解当前变更范围
2. 逐文件阅读变更内容
3. 从以下维度进行审查:
   - **正确性**: 逻辑错误、边界条件、空指针/None 处理
   - **安全性**: 注入风险、敏感信息泄露、权限检查
   - **可维护性**: 命名清晰度、函数长度、重复代码
   - **性能**: 不必要的计算、N+1 查询、内存泄漏风险
4. 给出具体的改进建议，包含代码示例

## 输出格式

对每个问题:
- 文件路径和行号
- 问题分类 (正确性/安全性/可维护性/性能)
- 具体描述
- 修复建议 (含代码)

最后给出整体评价和优先级排序。
