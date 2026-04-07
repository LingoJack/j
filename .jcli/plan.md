# Plan: 一次性完成 j-cli 的多 crate 重构方案

## 1. 改造目标

方案目标为一次性完成 `j-cli` 的最终架构改造，将现有“CLI 驱动的一体化程序”重构为“可复用内核 + CLI/TUI 适配层”的完整架构，完成以下结果：

1. 核心业务从 `src/command/**` 抽离到独立 crate
2. CLI 不再承载业务逻辑，只负责参数解析、调用 core、渲染输出
3. TUI 不再直接操作底层状态，只通过 core 服务完成业务
4. 配置、存储、错误、命令输入输出全部收敛为统一模型
5. alias、list、category、report、script、open、system、time、todo、chat 的核心能力全部归入 core
6. 旧的 handler 链路、直接打印式业务函数、半核心半界面的混合模块一次性清理

文档仅描述最终交付形态，不包含拆步实施路线。

---

## 2. 改造必要性

当前项目的根问题不是模块拆得不够细，而是边界定义错误：

1. `SubCmd -> handler -> handle_xxx()` 让 CLI 表达直接成为业务入口
2. 业务函数大量直接打印，导致逻辑无法被 GUI、remote、测试稳定复用
3. `YamlConfig` 同时承担配置模型、路径推导、状态存储、并发写入语义
4. `report`、`todo`、`chat` 等流程把“核心业务”和“TUI/交互表现”耦在一起
5. `src/command/**` 既像 application layer，又像 infra，又带 presentation 细节

拆步实施会直接导致以下问题：

1. 新旧两套调用链长期并存
2. 旧输出宏和新结构化输出双轨存在
3. core 会在很长时间内只是“新壳套旧逻辑”
4. 文档、代码、测试、认知模型全部被中间态污染

方案原则如下：

1. 只定义最终架构
2. 不引入中间态设计
3. 所有实现、测试、文档均以最终边界为准

---

## 3. 一次性重构后的目标结构

```text
j-cli/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── output.rs
│   ├── interactive/
│   └── tui/
├── crates/
│   ├── j-core/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── app/
│   │   │   │   ├── context.rs
│   │   │   │   ├── dispatcher.rs
│   │   │   │   └── runtime.rs
│   │   │   ├── command/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── alias.rs
│   │   │   │   ├── report.rs
│   │   │   │   ├── todo.rs
│   │   │   │   ├── chat.rs
│   │   │   │   ├── script.rs
│   │   │   │   ├── open.rs
│   │   │   │   ├── system.rs
│   │   │   │   ├── time.rs
│   │   │   │   └── update.rs
│   │   │   ├── service/
│   │   │   ├── store/
│   │   │   ├── config/
│   │   │   ├── model/
│   │   │   ├── error.rs
│   │   │   └── types.rs
│   └── j-tui/
│       └── src/
└── assets/
```

结构说明：

1. `j-core` 是唯一核心业务内核
2. 根 crate 保留 `j` 二进制与 CLI/TUI 入口
3. `j-tui` 只承载可复用 TUI 组件；如果当前没有必要单独发布，也可以先继续放在根 crate，但业务逻辑不能留在里面
4. `src/command/**` 旧目录不再作为业务层保留；能删除就删除，不能删除也只能保留为输入适配薄层

---

## 4. 清晰的分层定义

### 4.1 Presentation Layer

位置：

1. `src/main.rs`
2. `src/cli.rs`
3. `src/output.rs`
4. `src/interactive/**`
5. `src/tui/**` 或 `crates/j-tui/**`

职责：

1. 解析用户输入
2. 组装 `j_core::Command`
3. 调用 dispatcher 或 service
4. 将 `CommandOutput` 渲染为 CLI 文本、TUI 状态、交互结果

禁止事项：

1. 不允许直接读写业务文件
2. 不允许直接修改配置
3. 不允许直接执行 alias/report/todo/chat 的业务规则
4. 不允许直接依赖 `YamlConfig` 细节

### 4.2 Application Layer

位置：

1. `j_core::app::context`
2. `j_core::app::dispatcher`
3. `j_core::command::*`
4. `j_core::error`
5. `j_core::types`

职责：

1. 定义统一命令模型
2. 定义统一输出模型
3. 定义统一错误模型
4. 组织 service 调用与权限边界

### 4.3 Domain / Service Layer

位置：

1. `j_core::service::alias`
2. `j_core::service::report`
3. `j_core::service::todo`
4. `j_core::service::chat`
5. `j_core::service::script`
6. `j_core::service::open`
7. `j_core::service::system`
8. `j_core::service::time`
9. `j_core::service::update`

职责：

1. 只表达业务流程
2. 只返回结构化结果
3. 不关心终端打印、颜色、交互控件

### 4.4 Store / Infra Layer

位置：

1. `j_core::store::*`
2. `j_core::config::*`

职责：

1. 文件系统访问
2. Git 访问
3. 外部命令执行
4. 配置加载与保存
5. 路径与环境解析

---

## 5. 统一核心模型

### 5.1 Command

核心层不接受 `SubCmd`，只接受自己的命令模型：

```rust
pub enum Command {
    Alias(AliasCommand),
    Category(CategoryCommand),
    List(ListCommand),
    Report(ReportCommand),
    Todo(TodoCommand),
    Chat(ChatCommand),
    Script(ScriptCommand),
    Open(OpenCommand),
    System(SystemCommand),
    Time(TimeCommand),
    Update(UpdateCommand),
}
```

要求：

1. 每个子命令类型必须是领域化的，不携带 Clap 细节
2. 命令模型必须能被 CLI、TUI、remote 共用
3. 所有默认值、派生参数、语义校验都在 core 内部完成，不散落在 CLI 层

### 5.2 CommandOutput

不允许再用“打印字符串就是结果”的做法。

```rust
pub enum CommandOutput {
    Empty,
    Message(UserMessage),
    AliasList(Vec<AliasEntry>),
    AliasDetail(AliasEntry),
    CategoryList(Vec<CategoryEntry>),
    ReportLines(ReportLines),
    ReportWrite(ReportWriteResult),
    TodoSnapshot(TodoSnapshot),
    ChatSnapshot(ChatSnapshot),
    ScriptResult(ScriptResult),
    OpenResult(OpenResult),
    SystemInfo(SystemInfo),
    TimeInfo(TimeInfo),
    UpdateInfo(UpdateInfo),
}
```

要求：

1. output 必须完全脱离终端表现细节
2. output 必须足够结构化，CLI 只是 renderer，不再补业务判断
3. 不允许 `Message(String)` 成为主要返回类型，文字消息只能做补充，不可承载主语义

### 5.3 JError

```rust
pub enum JError {
    InvalidInput(String),
    NotFound(String),
    Conflict(String),
    PermissionDenied(String),
    Config(String),
    Io(String),
    ExternalCommand(String),
    Network(String),
    Serialization(String),
    Unsupported(String),
    Internal(String),
}
```

要求：

1. 错误分类必须足够支持 CLI/TUI/remote 的统一处理
2. 业务层不得直接输出错误文本到终端
3. CLI 层只负责把 `JError` 映射成用户可读提示

### 5.4 AppContext

`AppContext` 直接采用最终形态。

```rust
pub struct AppContext {
    pub paths: AppPaths,
    pub runtime: RuntimeOptions,
    pub config_store: Arc<dyn ConfigStore>,
    pub alias_store: Arc<dyn AliasStore>,
    pub report_store: Arc<dyn ReportStore>,
    pub todo_store: Arc<dyn TodoStore>,
    pub chat_store: Arc<dyn ChatStore>,
    pub git: Arc<dyn GitClient>,
    pub opener: Arc<dyn Opener>,
    pub clock: Arc<dyn Clock>,
}
```

要求：

1. 业务层只通过上下文依赖 infra
2. 所有核心流程都必须可测试、可替换、可脱离 CLI 运行
3. `YamlConfig::save()` 的 `flock` 语义必须保留在 `ConfigStore` 实现中，而不是暴露给业务层

---

## 6. 现有模块如何一次性归位

### 6.1 alias / category / list

这些模块全部下沉到 core，形成统一的 alias 领域：

1. `AliasService`
2. `CategoryService`
3. `AliasStore`
4. `AliasEntry`
5. `CategoryEntry`

CLI 不再保留 `handle_set`、`handle_remove`、`handle_list` 一类业务函数。

### 6.2 report

`report` 必须明确拆成三块：

1. `ReportService`
2. `ReportStore`
3. `ReportTuiAdapter`

归属原则：

1. 写日报、查日报、搜索、同步、Git push/pull、配置 URL 都属于 core
2. 打开编辑器、预填文本、渲染说明、TUI 输入循环属于 adapter

### 6.3 todo

`todo` 不能再维持“UI 驱动业务”的结构，必须拆成：

1. `TodoService`
2. `TodoStore`
3. `TodoSnapshot`
4. `TodoAction`

TUI 只能把用户操作翻译成 `TodoAction`，不能直接改状态文件。

### 6.4 chat

`chat` 属于高复杂度领域，但仍需纳入本轮完整边界。

必须收敛成：

1. `ChatService`
2. `SessionStore`
3. `ToolExecutionContext`
4. `ChatSnapshot`
5. `ChatAction`

要求：

1. 会话状态与消息存储进入 core
2. tool 执行上下文进入 core
3. CLI/TUI 只负责输入和呈现

### 6.5 script / open / system / time / update

这些属于标准命令域，全部进入 core，不再保留 CLI 直连逻辑。

其中：

1. `open` 使用 `Opener`
2. `system` 使用 `SystemService`
3. `update` 是否调用发布机制由 `UpdateService` 决定，CLI 只负责触发

---

## 7. 必须删除的旧结构

改造完成后，以下旧设计不应继续存在：

1. `SubCmd -> into_handler() -> handle_xxx()` 作为核心调用链
2. `src/command/**` 中直接打印并直接执行业务的函数
3. `info!()`、`error!()`、`usage!()` 深埋在 core 业务流程中
4. `YamlConfig` 被广泛以 `&mut` 形式穿透整个系统
5. 以宏生成 handler struct 作为业务注册中心的模式
6. report/todo/chat 模块内部直接调用 TUI 组件后再顺手完成业务落盘

可以保留的只有：

1. CLI 参数定义
2. 输入转换逻辑
3. 输出渲染逻辑
4. TUI adapter

---

## 8. 强约束

### 8.1 禁止伪重构

以下都算失败：

1. 把旧 `handle_xxx()` 原样复制到 `j-core`
2. `j-core` 里继续直接调用打印宏
3. `CommandOutput` 只是包一层字符串
4. `AppContext` 名字变了，但内部还是共享 `YamlConfig` 到处直接改
5. 旧 handler 系统还在承担主业务分发

### 8.2 禁止长期双轨

不接受“新旧链路并存一段时间”的设计。

完成标准是：

1. 新调用链是唯一真链路
2. 旧代码只允许作为已废弃包装，且应尽量删除
3. 文档、测试、实现全部基于同一套架构

### 8.3 core 禁止表现层依赖

`j-core` 不应依赖：

1. `colored`
2. `termimad`
3. `ratatui`
4. `tui-textarea`
5. 任何仅用于终端展示的 crate

### 8.4 TUI 禁止绕过 core

任何 TUI 操作都必须调用 core service，不允许：

1. 直接读配置文件
2. 直接写 todo/report/chat 状态
3. 直接拼装业务规则

---

## 9. 测试与验收

最终交付必须同时满足以下验收条件。

### 9.1 业务验收

1. `alias` 全量命令走 core
2. `list/category` 全量命令走 core
3. `report` 的核心命令走 core
4. `todo` 的增删改查走 core
5. `chat` 的会话状态管理走 core
6. `script/open/system/time/update` 走 core

### 9.2 架构验收

1. CLI 层不再持有核心业务逻辑
2. `j-core` 能被非 CLI 场景直接调用
3. 所有核心命令返回结构化 `CommandOutput`
4. 所有核心错误使用 `JError`
5. `YamlConfig` 的文件锁语义被 store 层完整保留

### 9.3 测试验收

至少应有：

1. alias 集成测试
2. report 集成测试
3. todo 服务测试
4. chat 会话存储测试
5. CLI 到 core 的命令映射测试
6. 输出 formatter 测试

### 9.4 行为验收

1. 常用命令对用户可见行为不倒退
2. `j report` 的交互流程仍可用
3. `j todo` 的 TUI 操作仍可用
4. `j chat` 的主流程仍可用

---

## 10. 风险与解决方式

### 风险 1：重构面太大

重构面较大属于既定事实。控制方式应为收紧完成定义与边界，而不是拆成多轮交付。

### 风险 2：chat/todo 边界复杂

解决方式：

1. 不降低目标
2. 只把 UI 细节留在 adapter
3. 先定义 snapshot/action/store，再让现有 UI 对接

### 风险 3：配置锁语义被破坏

解决方式：

1. 在 `ConfigStore` 内完整封装 `flock`
2. 业务层完全不直接接触保存细节
3. 为并发保存补测试

### 风险 4：输出结构设计过粗

解决方式：

1. 优先建领域输出类型
2. 禁止“先全用字符串兜底”
3. formatter 只能消费结构化字段，不能补推导主语义

---

## 11. 一次性交付清单

最终交付结果如下：

1. `crates/j-core/` 存在并承载全部核心业务
2. 根 crate 只负责 CLI/TUI/interactive 适配
3. 旧 handler 主链路不再存在
4. 旧命令模块中的直接打印式业务逻辑被清空或删除
5. report/todo/chat 的状态和业务归入 core
6. 核心层可被未来 GUI/remote 直接复用
7. 测试覆盖核心高风险路径

---

## 12. 方案结语

目标系统定义如下：

1. 边界清晰
2. 可复用
3. 可测试
4. 可被多宿主复用

文档仅保留最终形态、最终边界与最终验收标准。
