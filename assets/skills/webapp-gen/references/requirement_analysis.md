- 创建 `docs/` 目录（如无）
- 根据用户的一句话需求进行详细扩写
- 用 `Ask` 向用户确认，收集反馈并迭代
- 输出需求文档到 `docs/requirement.md`

需求文档必须包含如下内容：
- 标题：需求名称
- 一些预期的用例场景 Use Case
- 非功能性需求
写好之后，运行以下命令，以供用户查看需求文档
```bash
j open docs/requirement.md
```