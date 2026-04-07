# Plan: 将 j-cli 重构为可复用的多 crate 架构

## 1. 背景与结论

当前 `j-cli` 已经同时承担了三类职责：

1. CLI 参数解析与终端输出
2. 业务逻辑与文件系统/Git/网络操作
3. TUI 交互与状态管理

这三层目前混杂在同一个 crate 中，直接带来的问题不是“代码不优雅”，而是以下实际阻碍：

- 命令处理函数大量直接打印，无法稳定复用于 GUI、HTTP、Remote Agent 等宿主
- `SubCmd -> Handler -> handle_xxx()` 的链路本质仍是 CLI 驱动，业务层没有独立 API
- `YamlConfig::load/save()`、日报、todo、chat、script 等状态管理逻辑缺少统一错误模型
- TUI 模块与业务逻辑纠缠，导致“无界面复用”和“自动化测试”成本很高
- 后续新增 GUI/daemon/remote bridge 时，只能复制逻辑或继续放大耦合

结论：这次重构应该以“抽出稳定内核 API”为核心，而不是先追求彻底拆分 UI。优先把 CLI 从“直接执行业务”改为“适配层”，并建立可渐进迁移的 workspace 结构。

---

## 2. 当前架构现状

### 2.1 代码结构

当前主干结构大致如下：

```text
src/
├── main.rs                 # CLI 入口 + interactive fallback
├── lib.rs                  # 目前仅做少量导出
├── cli.rs                  # Clap SubCmd 定义
├── command/
│   ├── mod.rs              # dispatch(SubCmd, &mut YamlConfig)
│   ├── handler.rs          # SubCmd -> Box<dyn CommandHandler>
│   ├── alias.rs
│   ├── category.rs
│   ├── list.rs
│   ├── open.rs
│   ├── report.rs
│   ├── script.rs
│   ├── system.rs
│   ├── time.rs
│   ├── update.rs
│   ├── todo/
│   └── chat/
├── config/
│   └── yaml_config.rs      # 配置存储与部分路径逻辑
├── interactive/            # REPL
├── tui/                    # 通用编辑器/TUI 组件
└── util/
```

### 2.2 已有可复用资产

- `YamlConfig` 已具备独立模型价值，且有并发写锁处理
- `src/lib.rs` 已经存在，可作为抽象对外暴露起点
- `command/` 已按领域拆分，适合逐模块抽出 service
- `todo`、`chat` 内部已有状态对象和工具模块，具备进一步分层基础

### 2.3 关键问题

#### 问题 A：业务逻辑与输出耦合

例如 `report.rs`、`alias.rs`、`system.rs` 等模块直接依赖 `info!`、`error!`、`usage!` 宏输出。这意味着：

- GUI 无法拿到结构化结果
- 测试只能靠捕获 stdout/stderr
- 错误无法区分“用户输入问题”和“系统执行问题”

#### 问题 B：业务入口完全依赖 Clap 枚举

当前主链路是：

```text
Cli::parse()
  -> SubCmd
  -> into_handler()
  -> handle_xxx(...)
```

`SubCmd` 是 CLI 表达，不应成为核心业务 API。否则任何非 CLI 宿主都需要绕过或复刻这套表示。

#### 问题 C：配置、存储、执行上下文混在一起

当前大量函数签名类似：

```rust
fn handle_xxx(args..., config: &mut YamlConfig)
```

这会导致：

- 文件路径、运行模式、外部命令能力无法统一注入
- 未来想做 mock/store replacement 很困难
- chat/remote/GUI 对同一能力的调用缺少一致上下文

#### 问题 D：TUI 和核心能力没有明确边界

例如日报的 “打开编辑器 + 预填内容 + 写回文件” 是一个完整用户流，但其中只有一部分是核心能力；另一部分是纯 TUI 表现。目前两者耦在一起，不利于重用。

---

## 3. 重构目标

### 3.1 必须达成

1. CLI 变成适配层，而不是业务承载层
2. 核心命令返回结构化结果，不直接打印
3. 建立统一错误模型和运行时上下文
4. 支持 CLI、TUI、未来 GUI/remote 共用同一套 service API
5. 支持渐进迁移，期间保持现有命令行为基本可用

### 3.2 明确不在第一阶段解决

以下内容不应绑在首轮重构内，否则范围会失控：

- 不追求一次性把所有命令迁完
- 不强行把 chat TUI 整体抽成完全无 UI 的纯内核
- 不在首轮引入复杂 IoC/DI 框架
- 不优先做“所有输出格式完全统一”
- 不要求一开始就拆成很多细粒度 crate

原则：先稳定核心边界，再扩展。

---

## 4. 目标架构

### 4.1 推荐 workspace 结构

```text
j-cli/
├── Cargo.toml
├── src/                    # 现有 CLI crate，逐步瘦身
│   ├── main.rs
│   ├── cli.rs
│   ├── output.rs
│   └── interactive/
└── crates/
    ├── j-core/             # 核心领域与服务
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── app/
    │   │   │   ├── context.rs
    │   │   │   ├── dispatcher.rs
    │   │   │   └── output.rs
    │   │   ├── command/
    │   │   │   ├── mod.rs
    │   │   │   ├── alias.rs
    │   │   │   ├── report.rs
    │   │   │   ├── todo.rs
    │   │   │   └── ...
    │   │   ├── service/
    │   │   ├── store/
    │   │   ├── config/
    │   │   ├── error.rs
    │   │   └── types.rs
    │
    └── j-tui/              # 可选，后续抽离 TUI 复用组件
        └── src/
```

说明：

- `j-core` 是这次重构的核心产物
- `j-tui` 不必首轮创建，可以在 report/todo/chat 迁移后再评估
- 根 crate 仍保留二进制 `j`，避免一次性改发布方式

### 4.2 分层职责

#### Presentation Layer

- `src/main.rs`
- `src/cli.rs`
- `src/interactive/`
- 未来 GUI/remote server

职责：

- 解析输入
- 调用 `j-core`
- 把结构化结果渲染为终端文本 / TUI / GUI 状态

#### Application Layer

- `AppContext`
- `Command`
- `CommandDispatcher`
- `CommandOutput`
- `JError`

职责：

- 对外提供统一用法
- 路由命令到对应 service
- 约束输入输出与错误边界

#### Domain / Service Layer

- `AliasService`
- `ReportService`
- `TodoService`
- `ScriptService`
- `OpenService`
- `SystemService`

职责：

- 封装业务流程
- 不直接处理终端 UI
- 返回结构化结果

#### Store / Infra Layer

- `YamlConfigStore`
- `TodoStore`
- `ReportStore`
- `GitClient`
- `ShellOpener`

职责：

- 文件、Git、外部命令、路径管理
- 为 service 提供可替换依赖

---

## 5. 核心接口设计

### 5.1 命令模型

不要直接把现有 `SubCmd` 原样搬到 core。推荐做两层模型：

1. CLI 输入模型：保留在 `src/cli.rs`
2. Core 命令模型：定义在 `j-core`

示例：

```rust
pub enum Command {
    Alias(AliasCommand),
    Report(ReportCommand),
    Todo(TodoCommand),
    Script(ScriptCommand),
    Open(OpenCommand),
    System(SystemCommand),
}

pub enum ReportCommand {
    Write { content: String },
    Check { lines: usize },
    Search { query: String, lines: Option<usize>, fuzzy: bool },
    NewWeek { date: Option<NaiveDate> },
    Sync { date: Option<NaiveDate> },
    Push { message: Option<String> },
    Pull,
    SetUrl { url: String },
    Open,
}
```

这样做的好处：

- 领域更清晰，避免单个大 enum 持续膨胀
- GUI 和 remote 不需要理解 Clap 细节
- 未来命令权限、审计、序列化也更自然

### 5.2 输出模型

不建议只做一个 `Success { message }`。建议拆成“通用结果 + 领域结果”：

```rust
pub enum CommandOutput {
    Empty,
    Message(UserMessage),
    AliasList(Vec<AliasEntry>),
    ReportLines(ReportLines),
    TodoSnapshot(TodoSnapshot),
    OpenTarget(OpenResult),
    Version(VersionInfo),
}
```

要求：

- output 必须可被 CLI 渲染
- output 必须可被 GUI 直接消费
- output 不应夹带 ANSI、emoji、颜色等表现层细节

### 5.3 错误模型

建议使用：

```rust
pub enum JError {
    InvalidInput(String),
    NotFound(String),
    Conflict(String),
    Config(String),
    Io(String),
    ExternalCommand(String),
    Network(String),
    Unsupported(String),
    Internal(String),
}
```

同时提供：

```rust
impl JError {
    pub fn is_user_error(&self) -> bool { ... }
}
```

目的不是做“完美错误分类”，而是让 CLI/GUI 能区分：

- 可以直接提示用户修正
- 可以建议重试
- 需要记录技术细节

### 5.4 运行时上下文

不建议直接在 `AppContext` 中放 `Arc<Mutex<YamlConfig>>` 作为唯一入口。更合理的方向是：

```rust
pub struct AppContext {
    pub paths: AppPaths,
    pub config_store: Arc<dyn ConfigStore>,
    pub clock: Arc<dyn Clock>,
    pub opener: Arc<dyn Opener>,
    pub git: Arc<dyn GitClient>,
    pub runtime: RuntimeOptions,
}
```

首轮可以不全部 trait 化，但接口设计应朝这个方向保留空间。否则只是把全局耦合从 CLI 挪进 core。

---

## 6. 与当前代码的映射策略

### 6.1 第一批优先迁移模块

优先级建议：

1. `config/yaml_config.rs`
2. `command/alias.rs`
3. `command/list.rs`
4. `command/category.rs`
5. `command/report.rs`
6. `command/script.rs`
7. `command/open.rs`

原因：

- 这些模块边界相对清楚
- 对 chat/todo/TUI 的依赖较少或可局部切开
- 能尽快验证新架构，不会一开始卡死在 chat 系统

### 6.2 暂缓迁移模块

- `command/chat/**`
- `command/todo/ui.rs`
- `interactive/**`
- `tui/**`

这些模块应先做“调用 core 的适配层”，再逐步下沉能力，不适合首刀就深拆。

### 6.3 现有 dispatch 链路的演进

当前：

```text
SubCmd -> into_handler() -> handle_xxx()
```

目标：

```text
SubCmd -> TryInto<j_core::Command> -> dispatcher.dispatch()
       -> CommandOutput / JError -> output formatter
```

因此 `src/command/handler.rs` 最终大概率会被删除，至少不再作为核心分发机制。

---

## 7. 分阶段实施计划

## 阶段 0：基线与约束确认

目标：在动架构前，先锁定行为基线和迁移边界。

工作项：

1. 梳理命令清单，标记：
   - 纯命令型
   - TUI 型
   - 外部系统依赖型
2. 为高频命令补基础测试或最小行为快照：
   - `set/remove/rename/modify/list`
   - `report/check/search/reportctl`
   - `open`
3. 明确首轮不动 chat 主流程
4. 明确 workspace 发布策略不变，`j` 仍为唯一对外二进制

验收标准：

- 有一份迁移范围表
- 至少覆盖 alias/report 主路径的行为测试
- 能说明哪些模块禁止在首轮深改

## 阶段 1：建立 workspace 与 j-core 骨架

目标：不改行为，只先抽出承载层。

工作项：

1. 把根 `Cargo.toml` 调整为真正的 workspace
2. 新建 `crates/j-core`
3. 在 `j-core` 中创建：
   - `command`
   - `error`
   - `app/context`
   - `app/dispatcher`
   - `types`
4. 将 `YamlConfig` 及其直接相关路径逻辑迁入 `j-core::config`
5. 根 crate 改为依赖 `j-core`

验收标准：

- `cargo build` 通过
- 现有 CLI 行为不变
- `src/main.rs` 已依赖 `j-core` 的配置类型，而非本地定义

## 阶段 2：迁移 alias/list/category 为首批服务

目标：验证“结构化输入/输出”链路。

工作项：

1. 定义：
   - `AliasCommand`
   - `AliasService`
   - `AliasEntry`
2. 把别名增删改查和分类逻辑迁到 `j-core`
3. CLI 层新增：
   - `impl TryFrom<SubCmd> for Command`
   - `print_output(CommandOutput)`
4. 保留旧命令文件作为薄包装或直接删除

验收标准：

- `j set/remove/rename/modify/list/find/note/denote` 均走 `j-core`
- CLI 不再在 alias 业务流程中直接调用 `handle_xxx`
- 错误信息与现有行为基本等价

## 阶段 3：迁移 report，拆开业务与 TUI

目标：解决最典型的 I/O 耦合样板。

工作项：

1. 将 `report.rs` 拆成三层：
   - `ReportService`: 写日报、查最近内容、搜索、元数据更新
   - `ReportStore`: 文件读写、settings.json、路径处理
   - `ReportTuiAdapter`: 预填内容、编辑器启动、提交回写
2. 明确哪些逻辑属于核心：
   - `write/check/search/new/sync/push/pull/set-url/open`
3. 明确哪些逻辑属于表现层：
   - 打开 Markdown 编辑器
   - 渲染提示文本
4. 为 Git push/pull 建立单独错误映射

验收标准：

- `reportctl` 主流程可经由 `j-core`
- `j report` 的无参数 TUI 流程仍可用
- `j-core` 可以在不启动 TUI 的情况下完成日报读写和查询

## 阶段 4：迁移 script/open/system/time/update

目标：完成常规命令层迁移。

工作项：

1. 抽出 `ScriptService`
2. 抽出 `OpenService`
3. 抽出 `SystemService`
4. 评估 `UpdateService` 是否仍保留在 CLI 层

说明：

- `update` 依赖发布与自更新机制，允许暂时保留在 CLI
- `time` 比较轻，可以最后迁

验收标准：

- 非 TUI 常规命令基本均已通过 `j-core`
- `src/command/` 中残留的业务逻辑明显减少

## 阶段 5：todo/chat 的边界治理

目标：先收边界，再决定是否拆 crate。

工作项：

1. 为 todo 提炼：
   - `TodoService`
   - `TodoStore`
   - `TodoSnapshot`
2. TUI 仅负责状态展示和按键交互
3. chat 不做一次性迁移，只先拆：
   - 配置加载
   - 会话存储
   - tool 执行上下文
4. remote bridge 与本地 chat 共用同一核心会话接口

验收标准：

- todo 的基础增删改查可脱离 TUI 使用
- chat 至少能把一部分“非 UI 状态”下沉到 core

---

## 8. 设计原则与实现约束

### 8.1 渐进迁移，不做一次性大爆炸

每迁一个领域，都应保持：

- 旧命令入口仍可运行
- 调用链可双轨存在一段时间
- 迁完一个领域再删旧桥接代码

### 8.2 避免“伪抽象”

以下做法看起来像重构，实际只是换名字：

- 把 `handle_xxx()` 复制到 `j-core` 但继续直接打印
- 把 `SubCmd` 直接公开给 `j-core`
- 把 `YamlConfig` 放进 `Mutex` 后继续到处可变借用
- 建一个 `Service` trait，但所有命令都塞进同一个巨大 `match`

重构是否成功的判断标准是：CLI 拿掉后，核心逻辑还能独立被别的宿主调用。

### 8.3 优先稳定数据边界，而不是 trait 数量

首轮重点不是“接口设计得多优雅”，而是：

- 输入边界清楚
- 输出边界清楚
- 错误边界清楚
- 存储边界清楚

只在确实需要替换实现时再引入 trait。

---

## 9. 风险与应对

### 风险 1：重构范围失控

表现：

- 一开始就试图一起改 chat、todo、interactive、tui

应对：

- 锁定首轮只迁 alias/report/常规命令
- 对 chat 仅治理边界，不做深拆

### 风险 2：行为回归

表现：

- 原本 CLI 提示、默认值、文件路径、日期逻辑发生变化

应对：

- 在阶段 0 建立回归测试
- 对 report 和 alias 做 golden/集成测试

### 风险 3：核心层反而继续持有终端依赖

表现：

- `j-core` 继续依赖 `colored`、`termimad`、TUI 组件

应对：

- 明确 `j-core` 禁止引入终端表现依赖
- 渲染相关代码留在 CLI/TUI 层

### 风险 4：配置并发语义被破坏

表现：

- 迁移后 `YamlConfig::save()` 的锁语义丢失

应对：

- 将现有锁逻辑原样保留并补测试
- 在 store 层封装保存行为，避免业务层绕写

---

## 10. 验收标准

本次重构完成的最低标准不是“代码更整齐”，而是以下几点同时成立：

1. `j-core` 能独立承载 alias、report、list/category 等核心命令
2. CLI 只负责输入解析和输出格式化
3. 核心命令返回结构化 `CommandOutput` 与 `JError`
4. `j report` 的 TUI 流程仍保持可用
5. 至少有一组集成测试覆盖 alias/report 关键路径
6. `src/command/handler.rs` 不再是核心业务组织方式

加分项：

- todo 基础能力完成下沉
- remote/chat 复用了一部分 core 能力
- 未来 GUI 可以直接链接 `j-core`

---

## 11. 推荐落地顺序

按实际收益和改动风险，推荐顺序如下：

1. 建 workspace 与 `j-core` 骨架
2. 迁 `YamlConfig`
3. 迁 alias/list/category
4. 建 CLI `SubCmd -> Command` 转换层
5. 建统一 output formatter
6. 迁 report 核心逻辑
7. 拆 report TUI 适配层
8. 迁 script/open/system/time
9. 评估 todo
10. 最后处理 chat/remote 深层抽象

---

## 12. 建议的第一批具体任务

如果按这份计划开始实施，建议先开以下任务单：

### Task 1：workspace 与 j-core 初始化

- 调整根 `Cargo.toml`
- 新建 `crates/j-core`
- 搭建最小 `lib.rs` / `error.rs` / `command/mod.rs`

### Task 2：迁移配置模型

- 把 `src/config/yaml_config.rs` 移入 `j-core`
- 修复根 crate 引用
- 确保 `cargo build` 通过

### Task 3：实现第一版 Command/Output/Error

- 定义 alias/report 的命令模型
- 增加 dispatcher
- 增加 CLI output formatter

### Task 4：迁移 alias 全链路

- `set/remove/rename/modify/list/find/note/denote`
- 让 CLI 完整走 `j-core`
- 补集成测试

### Task 5：迁移 report 非 TUI 核心

- `write/check/search/reportctl`
- 先不动编辑器界面，只保留适配桥

---

## 13. 最终判断

这次重构是必要的，但不应该被定义成“全面重写”。正确方向是：

- 先抽核心 API
- 再瘦 CLI
- 最后治理 TUI/chat 边界

如果按这个顺序执行，重构收益会很快体现出来；如果一开始就追求把所有模块彻底纯化，大概率会陷入长时间分支开发和高回归风险。
