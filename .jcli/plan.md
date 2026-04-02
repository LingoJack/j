# Plan: 对比 Claude Code 和 jcli 工具描述系统的差异，生成详细报告

## 调研结果

### 1. 架构对比

| 维度 | Claude Code | jcli |
|------|-------------|------|
| 文件结构 | 每个工具独立 `prompt.ts` 文件 | Rust 内联 `description()` 方法 |
| 代码位置 | `src/tools/<ToolName>/prompt.ts` | `src/command/chat/tools/<tool>.rs` |
| 描述函数 | `getDescription()` 或 `getSimplePrompt()` | `impl Tool for XxxTool { fn description() }` |

### 2. 工具描述详细程度对比

#### Bash 工具

**Claude Code (~370行)**:
```typescript
// 包含:
- 工具偏好指导 (Use Read/Write/Edit, not cat/sed/echo)
- 后台任务处理 (run_in_background)
- Git commit/PR 详细流程 (~80行)
- Sandbox 配置详情 (~80行)
- 多命令执行策略 (&& vs ; vs parallel)
- Timeout 配置说明
- 睡眠命令避免指南
```

**jcli (~12行)**:
```rust
fn description(&self) -> &str {
    r#"
    Execute shell commands on the current system...
    Important limitations:
    - Interactive commands are not supported
    - Commands that exceed the timeout...
    Usage tips:
    - Chain independent commands with &&...
    - Use absolute paths...
    "#
}
```

#### Grep 工具

**Claude Code (~17行)**:
```typescript
export function getDescription(): string {
  return `A powerful search tool built on ripgrep
  Usage:
  - ALWAYS use Grep for search tasks. NEVER invoke grep or rg
  - Supports full regex syntax...
  - Filter files with glob parameter...
  - Output modes: "content", "files_with_matches", "count"
  - Use Agent tool for open-ended searches...
  `
}
```

**jcli (~13行)**:
```rust
fn description(&self) -> &str {
    r###"
    - Powerful regex-based search tool for searching within file contents
    - Supports full regex syntax...
    - Filter files with the glob parameter...
    - Output modes: "content", "files_with_matches", "count"
    - Supports pagination...
    "###
}
```

#### Read 工具

**Claude Code (~50行)**:
```typescript
export function renderPromptTemplate(...): string {
  return `Reads a file from the local filesystem...
  Usage:
  - The file_path parameter must be an absolute path...
  - By default, it reads up to 2000 lines...
  - Results are returned using cat -n format...
  - This tool allows Claude Code to read images...
  - This tool can read PDF files...
  - This tool can read Jupyter notebooks...
  - You will regularly be asked to read screenshots...
  `
}
```

**jcli (1行)**:
```rust
fn description(&self) -> &str {
    "Read local file contents and return with line numbers. 
     Supports reading by line range via offset and limit parameters. 
     Can also read image files (png/jpg/gif/webp/bmp)..."
}
```

### 3. 参数 Schema 对比

两者都使用 JSON Schema 格式，差异较小：

| 参数描述 | Claude Code | jcli |
|---------|-------------|------|
| pattern | "Regex pattern to search for (e.g. \"log.*Error\", \"function\\\\s+\\\\w+\")" | 相同 |
| path | "File or directory path to search. Defaults to current working directory if not specified. Important: omit this field if not needed" | 类似 |
| timeout | "Timeout in seconds, default 120, max 600..." | "Timeout in seconds, default 120, max 600..." |

### 4. Claude Code 独有特性

1. **Git 工作流指导** (在 Bash 工具描述中):
   - Git Safety Protocol
   - Commit 创建流程
   - PR 创建流程
   - 使用 HEREDOC 格式化 commit message

2. **Sandbox 配置**:
   - 动态注入沙箱限制说明
   - `dangerouslyDisableSandbox` 参数使用指导

3. **工具偏好指南**:
   ```
   File search: Use Glob (NOT find or ls)
   Content search: Use Grep (NOT grep or rg)
   Read files: Use Read (NOT cat/head/tail)
   Edit files: Use Edit (NOT sed/awk)
   ```

4. **Agent 工具集成**:
   - `Use Agent tool for open-ended searches requiring multiple rounds`

### 5. jcli 特点

1. **简洁性**: 描述更短，依赖 LLM 自行推断
2. **参数内描述**: 参数说明在 JSON Schema 中更详细
3. **Plan Mode 集成**: 工具注册时包含 plan mode 白名单检查

### 6. Token 消耗估算

| 工具 | Claude Code | jcli | 差异 |
|------|-------------|------|------|
| Bash | ~1500 tokens | ~150 tokens | 10x |
| Grep | ~200 tokens | ~180 tokens | 1.1x |
| Read | ~400 tokens | ~80 tokens | 5x |
| Write | ~300 tokens | ~100 tokens | 3x |

### 7. 建议改进方向

**短期优化** (低风险):
1. 为 Bash 工具添加工具偏好指南
2. 为 Read 工具添加多模态能力说明 (图片、PDF)
3. 统一输出格式说明 (如 cat -n 格式)

**中期优化** (中等风险):
1. 添加 Git 工作流指导 (可抽取为 skill)
2. 添加 Sandbox 相关说明
3. 添加 Agent 工具协作指南

**长期优化** (需要架构支持):
1. 分离 prompt 文件，支持动态加载
2. 支持用户自定义工具描述覆盖
3. 支持 skill 系统注入工具使用指南

## 结论

Claude Code 的工具描述系统更成熟，包含大量使用指南和最佳实践，但 token 消耗较高。jcli 的描述更简洁，适合快速迭代，但缺少使用指导。

建议采用渐进式优化策略，先补充关键的缺失特性，再考虑架构层面的改进。

## 下一步行动

- [ ] 确认是否需要更新 jcli 工具描述
- [ ] 选择优化优先级 (Bash/Read/Grep)
- [ ] 确定实现方式 (硬编码 vs 配置文件)
