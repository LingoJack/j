# README 重构计划

## 问题分析

当前 README 存在的问题：
1. **像技术说明书**：内容冗长、格式单调，全是命令和配置的罗列
2. **缺少项目展示**：没有 Logo、截图、Badge 等视觉元素
3. **缺少快速上手体验**：用户打开后不能 30 秒内知道这是什么、怎么用
4. **网站链接缺失**：没有指向 web 文档站点的入口

## 设计参考

参考 DragonOS 的 README 风格：
- 顶部：Logo + 一句话介绍 + Badge 徽章
- 特性亮点：图标 + 简洁描述（网格布局）
- 快速安装：折叠的安装命令，分平台
- 截图/动图展示：让用户直观看到效果
- 链接区：文档网站、贡献指南等

## 新 README 结构

```
1. Header
   - Logo (favicon SVG 或文字 Logo "j")
   - 标语："AI 驱动的命令行工作台"
   - Badge 徽章：Rust 版本 / License / 版本号
   - 官网链接按钮

2. 功能亮点（Features）
   - 分两列网格，6 个核心功能点，每个用 emoji 图标 + 标题 + 一句话描述
   - 覆盖：AI Chat、日报周报、Todo、别名管理、脚本管理、工具生态

3. AI Chat 截图展示
   - 嵌入 web/public/pics/jcli-ai/ 中的截图
   - 展示 AI 对话界面效果

4. 快速开始（Quick Start）
   - 一行安装命令（curl / PowerShell），分平台 tab
   - 最简配置引导（设置 API Key）

5. 核心命令速览
   - 表格形式，命令 => 说明，10 行以内
   - 不展开细节，引导到文档网站

6. 文档 & 链接
   - 官方文档网站链接（GitHub Pages）
   - 技术栈说明
   - License / Author 信息
```

## 关键改动点

| 原位置 | 改动 |
|--------|------|
| 长篇配置说明 | 移到 web 文档，README 只保留最简配置 |
| 所有命令详细用法 | 改为速览表格，详细内容指向 web 文档 |
| 无视觉元素 | 添加 Logo、Badge、AI 截图 |
| 无网站链接 | 添加 GitHub Pages 文档链接 |
| 无特性总览 | 新增 6 大特性亮点区域 |

## 网站地址

根据 vite.config.ts 的 `base: '/j/'` 和 build 输出到 `docs/` 目录，网站地址为：
`https://lingojack.github.io/j/`

（需确认实际部署地址，规划中先用此占位）

## 实施步骤

1. 备份当前 README.md 为 README.old.md
2. 编写新的 README.md（按上述结构）
3. 确认图片路径可用（pics 已在 web/public/ 中，README 引用时用 GitHub 相对路径或绝对 URL）
