# Plan: 一次性完成 j-cli 的多 crate 重构方案

## 1. 改造目标

方案目标为一次性完成 `j-cli` 的最终架构改造，将现有"CLI 驱动的一体化程序"重构为"可复用内核 + CLI/TUI 适配层"的完整架构，完成以下结果：

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
4. `report`、`todo`、`chat` 等流程把"核心业务"和"TUI/交互表现"耦在一起
5. `src/command/**` 既像 application layer，又像 infra，又带 presentation 细节

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
│   ├── tui/
│   └── command/
│       ├── chat/          # Chat TUI 适配层
│       ├── todo/          # Todo TUI 适配层
│       └── report.rs      # Report TUI 适配层
├── crates/
│   └── j-core/
│       ├── src/
│       │   ├── lib.rs
│       │   ├── context.rs     # AppContext
│       │   ├── error.rs       # JError
│       │   ├── types.rs       # 通用类型
│       │   ├── config/        # 配置存储（封装 YamlConfig）
│       │   ├── command/       # 命令模型定义
│       │   │   ├── mod.rs
│       │   │   ├── alias.rs
│       │   │   ├── report.rs
│       │   │   ├── todo.rs
│       │   │   ├── chat.rs
│       │   │   ├── script.rs
│       │   │   ├── open.rs
│       │   │   ├── system.rs
│       │   │   ├── time.rs
│       │   │   └── update.rs
│       │   ├── service/       # 业务逻辑
│       │   └── store/         # 数据存储
└── assets/
```

结构说明：

1. `j-core` 是唯一核心业务内核
2. 根 crate 保留 `j` 二进制与 CLI/TUI 入口
3. **不拆 `j-tui` crate**：TUI 组件（editor, todo UI, chat UI）与具体功能紧耦合，没有独立发布需求，保留在根 crate 即可
4. `src/command/` 下简单命令（alias, list, category, time, script）的旧文件直接删除；有 TUI 交互的模块（chat, todo, report）保留为纯适配层

---

## 4. 清晰的分层定义

### 4.1 Presentation Layer

位置：

1. `src/main.rs`
2. `src/cli.rs`
3. `src/output.rs`
4. `src/interactive/**`
5. `src/tui/**`
6. `src/command/`（仅 TUI 适配层：chat, todo, report）

职责：

1. 解析用户输入
2. 组装 `j_core::Command`
3. 调用 core service
4. 将 `CommandOutput` 渲染为 CLI 文本、TUI 状态、交互结果

禁止事项：

1. 不允许直接读写业务文件
2. 不允许直接修改配置
3. 不允许直接执行 alias/report/todo/chat 的业务规则
4. 不允许直接依赖 `YamlConfig` 细节

### 4.2 Application Layer

位置：

1. `j_core::command::*`
2. `j_core::context`
3. `j_core::error`
4. `j_core::types`

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

```rust
pub enum CommandOutput {
    Empty,
    Message(String),
    AliasList(Vec<AliasEntry>),
    AliasResult(AliasOpResult),
    CategoryList(Vec<CategoryEntry>),
    CategoryResult(CategoryOpResult),
    ReportLines(Vec<ReportLine>),
    ReportWriteResult(ReportWriteResult),
    TodoSnapshot(TodoSnapshot),
    ChatSnapshot(ChatSnapshot),
    ScriptResult(ScriptResult),
    OpenResult(OpenResult),
    SystemInfo(SystemInfo),
    TimeInfo(TimeInfo),
    UpdateInfo(UpdateInfo),
}
```

务实原则：

1. `Message(String)` 允许存在，用于简单的操作确认消息（如"别名已删除"）
2. `Message(String)` 不应承载主业务数据（如列表、搜索结果等），这些必须用结构化类型
3. CLI 的 `output.rs` 负责将 `CommandOutput` 渲染为用户可读文本，是唯一的渲染入口

### 5.3 JError

```rust
#[derive(Debug, thiserror::Error)]
pub enum JError {
    #[error("无效输入: {0}")]
    InvalidInput(String),
    #[error("未找到: {0}")]
    NotFound(String),
    #[error("已存在: {0}")]
    AlreadyExists(String),
    #[error("配置错误: {0}")]
    Config(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("外部命令失败: {0}")]
    ExternalCommand(String),
    #[error("网络错误: {0}")]
    Network(String),
    #[error("序列化错误: {0}")]
    Serialization(String),
}
```

要求：

1. 使用 `thiserror` 派生，不手写 Display
2. 错误类型按实际使用场景定义，不为假想需求预设（如无 `PermissionDenied`、`Conflict`、`Unsupported`、`Internal`，当前代码中没有这些场景）
3. `AlreadyExists` 覆盖 alias set 重复等高频场景
4. 业务层不得直接输出错误文本到终端
5. CLI 层只负责把 `JError` 映射成用户可读提示

### 5.4 AppContext

```rust
pub struct AppContext {
    pub paths: AppPaths,
    pub config: ConfigStore,
}
```

精简设计原则：

1. AppContext 只持有真正需要全局共享的依赖：路径配置和配置存储
2. 各 Service 函数按需接收具体参数，不把所有依赖塞进一个上帝对象
3. `ConfigStore` 内部封装 `flock` 语义，业务层不直接接触保存细节

不使用 `Arc<dyn Trait>` 的原因：

1. **Alias 没有独立存储**——数据在 YamlConfig 里，通过 `ConfigStore` 访问即可
2. **Report/Todo 存储就是文件系统操作**——不需要 trait 抽象，用普通函数即可
3. **GitClient / Opener / Clock trait**——当前分别只在 report push/pull、open 命令、无处使用，不值得预设 trait
4. 如果未来某个依赖确实需要 trait 抽象（比如要给 Git 操作写测试），按需添加

---

## 6. 现有模块如何一次性归位

### 6.1 alias / category / list

这些模块全部下沉到 core：

1. `AliasService`：set, remove, rename, modify
2. `CategoryService`：note, denote
3. `ListService`：list

不需要独立的 `AliasStore` trait。Alias 数据存储在 YamlConfig 中，通过 `ConfigStore` 即可访问。

CLI 不再保留 `handle_set`、`handle_remove`、`handle_list` 一类业务函数，这些文件直接删除。

### 6.2 report

`report` 必须明确拆成两块：

1. **`ReportService`（进 core）**：写日报、查日报、搜索、同步、Git push/pull、配置 URL
2. **Report TUI 适配层（留根 crate）**：打开 TUI 编辑器、预填文本、渲染交互

不需要独立的 `ReportStore` trait。Report 数据是文件系统上的 markdown 文件，service 函数直接操作文件路径即可。

### 6.3 todo

`todo` 不能再维持"UI 驱动业务"的结构，必须拆成：

1. **`TodoService`（进 core）**：TodoItem/TodoList 的 CRUD、持久化、筛选
2. **Todo TUI 适配层（留根 crate）**：TUI 渲染、键盘交互、列表状态

`TodoSnapshot` 和 `TodoAction` 定义在 core 中，TUI 层只能把用户操作翻译成 `TodoAction`，不能直接改状态文件。

### 6.4 chat

Chat 模块 ~30K 行，是项目中复杂度最高的部分，有自己完整的内部架构（ChatApp 状态机、Action 驱动、流式处理、15+ 工具、权限系统、hook 系统）。

迁移策略需要区别对待：

**必须进 core 的部分：**
1. 会话状态与消息存储（`storage.rs`、`archive.rs` 的数据操作）
2. Agent config 的加载/保存
3. `ChatSnapshot`、`ChatAction` 模型定义
4. 会话管理逻辑（创建、恢复、清空）

**保留在根 crate 的部分：**
1. TUI 渲染和事件循环（`tui_loop.rs`、`ui/`）
2. 流式响应处理（与 TUI 紧耦合）
3. 工具系统（15+ 工具与 agent 循环紧耦合，强行拆出 core 只会制造无意义的间接层）
4. markdown 解析和语法高亮（纯表现层）
5. theme、render_cache（纯表现层）

**需要评估的部分：**
1. `agent.rs`（Agent 循环）：如果未来有非 TUI 场景需要复用 agent 循环（如 remote server 独立部署），则应进 core；当前 remote server 已内嵌在 chat 模块中，暂可留在根 crate
2. `api.rs`（API 客户端）：同上
3. `permission.rs` / `hook.rs`：这些是 agent 运行时的基础设施，如果 agent 进 core 则一起进

要求：
1. 会话状态与消息存储进入 core
2. TUI 层通过 core service 访问会话数据，不直接读写文件
3. CLI/TUI 只负责输入和呈现

### 6.5 script / open / system / time / update

这些属于标准命令域，全部进入 core，不再保留 CLI 直连逻辑。

其中：

1. `open` 命令的"打开什么"逻辑进 core，系统 `open` 调用也进 core
2. `system` 中 `contain`、`change`、`log` 进 core，`clear`/`help`/`version`/`completion` 可以留在 CLI 层（纯展示逻辑）
3. `update` 的版本检查和更新执行进 core，CLI 只负责触发和显示进度

---

## 7. 必须删除的旧结构

改造完成后，以下旧设计不应继续存在：

1. `SubCmd -> into_handler() -> handle_xxx()` 作为核心调用链
2. `src/command/` 中简单命令的直接打印式业务函数
3. `info!()`、`error!()`、`usage!()` 深埋在 core 业务流程中
4. `YamlConfig` 被广泛以 `&mut` 形式穿透整个系统
5. 以宏生成 handler struct 作为业务注册中心的模式（`command_handlers!` 宏）
6. todo 模块内部直接调用文件读写完成业务落盘

可以保留的只有：

1. CLI 参数定义（`cli.rs`）
2. 输入转换逻辑（SubCmd -> Command 的映射）
3. 输出渲染逻辑（`output.rs`）
4. TUI 适配层（chat, todo, report 的 UI 代码）

---

## 8. 强约束

### 8.1 禁止伪重构

以下都算失败：

1. 把旧 `handle_xxx()` 原样复制到 `j-core`
2. `j-core` 里继续直接调用打印宏
3. `CommandOutput` 只是包一层字符串（但 `Message(String)` 用于简单确认消息是允许的）
4. `AppContext` 名字变了，但内部还是共享 `YamlConfig` 到处直接改
5. 旧 handler 系统还在承担主业务分发

### 8.2 禁止长期双轨

不接受"新旧链路并存一段时间"的设计。

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
5. `YamlConfig` 的文件锁语义被 `ConfigStore` 完整保留

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

### 风险 2：chat 模块复杂度高

Chat ~30K 行，有独立的内部架构。解决方式：

1. 明确区分"必须进 core"和"保留在根 crate"的部分（见 6.4 节）
2. 数据层和配置层必须进 core，TUI 和工具系统允许留在根 crate
3. 不强求 100% 业务进 core，但数据访问必须通过 core

### 风险 3：配置锁语义被破坏

解决方式：

1. 在 `ConfigStore` 内完整封装 `flock`
2. 业务层完全不直接接触保存细节
3. 为并发保存补测试

### 风险 4：输出结构设计过粗

解决方式：

1. 优先建领域输出类型
2. `Message(String)` 仅用于操作确认，不承载主业务数据
3. formatter 只能消费结构化字段

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
