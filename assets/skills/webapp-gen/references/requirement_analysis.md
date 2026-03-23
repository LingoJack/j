# 需求分析阶段
使用 `TodoWrite` 跟踪以下待办依次执行：
## 初始化项目，在工作目录执行
	```bash
	python3 <skill_base_path>/scripts/init_project.py
	```

## 分析需求文档
根据用户的一句话需求进行详细扩写
需求文档必须包含如下内容：
- 标题：需求名称
- 一些预期的用例场景 Use Case
- 非功能性需求
写好之后，运行以下命令，以供用户查看需求文档
```bash
j code docs/requirement.md
```

## 获取需求文档反馈
用 `Ask` 向用户确认，收集反馈并迭代
输出需求文档到 `docs/requirement.md`


