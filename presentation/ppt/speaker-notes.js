// 演讲稿数据 · 从 speaker-notes.xml 同步
// 编辑方式：改 speaker-notes.xml，再同步到这里
var SPEAKER_NOTES_XML = `
<?xml version="1.0" encoding="utf-8"?>
<speaker-notes>

<note slide="Cover">
各位评委老师好，我的毕设主题是《》
</note>

<note slide="项目">
这是我的项目信息。指导老师是曾彬老师，项目名称是"基于 LLM Agent 的编程辅助工具设计与实现"，工具叫 j-cli，用 Rust 写的命令行工具。
</note>

<note slide="Agenda">
先看一下今天的议程，总共六个部分。

第一部分讲背景——为什么我们要从 ChatBot 走向 Agent，这中间的核心差异是什么。

第二部分是整个项目的核心——上下文窗口治理。LLM 的窗口有限，Agent 要长期运行就必须有一套机制来管理"往窗口里塞什么、丢掉什么"。

第三部分讲工具体系，从最基础的 Read/Write/Bash 几个工具开始，一步步说明每个新工具引入是为了解决什么问题。

第四部分讲多 Agent 协作——Teammate 和 SubAgent 怎么配合工作，以及错误处理和重试机制。

第五部分讲 Skill 系统——怎么把工程化能力封装成按需加载的技能包。

最后收尾总结。
</note>

<note slide="ChatBot vs Agent">
先从最根本的问题开始——ChatBot 和 Agent 的区别到底是什么？

大家回想一下 2023、2024 年用 GPT 的场景。我们有 bug，需要自己去收集代码、报错日志、运行环境信息，然后复制粘贴到 ChatBot 里，ChatBot 给出建议，我们再自己去改代码。

这个过程里，人承担了两个关键角色：上下文收集者和物理世界与逻辑世界的桥梁。"物理世界"就是你的代码、文件系统、终端；"逻辑世界"就是 LLM 的思考空间。

而 Agent 的核心转变就是：通过上下文窗口的治理和工具的使用，Agent 自己承担了这些角色，人被抽离了出来。

但这带来一个新的挑战——Agent 怎么在一个有限的上下文窗口里，长期稳定地工作？这就是我整个毕设要解决的核心问题。
</note>

<note slide="Function Call">
在讲 ReAct 循环之前，先讲一个前置概念——Function Calling。

LLM 本质上是一个文本预测器。你给它一段文字，它接着往下写。它不能直接操作文件系统、不能跑命令、不能访问网络。那 Agent 是怎么"做事"的？

先理解 LLM 的输入结构。每次请求，LLM 收到的是一组 messages。分三种角色：System 是全局指令，包括身份设定、行为规则、工具的 Schema 定义——整轮对话固定不变；User 是用户输入，也包括系统动态注入的提醒；Assistant 是 LLM 自己的回复，可能是纯文本，也可能是 tool_calls。

这个 messages 数组就是 LLM 能看到的全部世界。它没有记忆，没有硬盘，上下文窗口就是它的"工作记忆"。后面讲压缩、讲 system-reminder，都是在操作这个 messages 数组。

理解了输入结构，再看 Function Calling。思路分四步：第一步，运行时把每个工具的名称、功能描述、参数格式注册到 System 里。比如"有一个叫 Bash 的工具，接收 command 字符串"。

第二步，LLM 根据上下文推理，判断需不需要调工具。如果需要，它不执行任何操作，只输出一段 tool_calls JSON——工具名 + 参数值。右边上面那个例子就是 LLM 输出的 function call：调 Bash，命令是 ls -F。

第三步，运行时拿到 JSON，在真实环境里执行。比如真的去跑 ls 命令。

第四步，把执行结果以 User 角色回灌到 messages。LLM 看到结果后继续推理——可能够了直接回答，也可能需要再调一个工具。这就是循环。

关键点是：LLM 不直接执行任何操作，它只产出"意图"——一段 JSON。运行时代替它完成真实动作。

下一页讲 ReAct 循环——把 Function Calling 放到一个更大的推理框架里看。
</note>

<note slide="ReAct · concept">
Agent 的心脏是 ReAct 循环——Reasoning + Acting 的合写。

每一轮分四步：Thought 看上下文决定动作；Action 生成结构化的 tool call；Observation 运行时真正执行工具、把结果回灌窗口；Update 进入下一轮。

这页讲的是概念。后面六页我会用一个真实任务，逐帧演示这个循环到底怎么跑、窗口怎么变化。
</note>

<note slide="ReAct · user">
第一帧——用户给一个开放性需求："总结一下 src/command/chat 目录的结构和可优化的模块划分"。

注意这句话不是指令。用户没说"跑 ls"也没说"读哪个文件"。Agent 要把这句模糊的需求翻译成一连串具体的工具调用。

这时候窗口里只有系统提示和这一行用户输入，占用 6%。下一帧 Agent 开始思考。
</note>

<note slide="ReAct · action">
第二帧——Agent 想了一下。它的思考过程也会被记录进窗口，这条 thought 写得很短："先列出 chat/ 下的文件分布"。

然后它生成一个 tool call。注意这不是简单的"我想跑这个命令"，而是一个结构化的 JSON，运行时能直接拿去执行。参数明确包含 command 和 description 两个字段。

窗口从 6% 涨到 9%——thought 加 tool call 占了 3%。这个增长很小，因为消息很短。但你想，跑十几轮之后会怎样？这就是上下文压力的来源。
</note>

<note slide="ReAct · observation">
第三帧——运行时执行 ls，把结果原样写回到对话上下文，作为一条新消息。

这是 ReAct 最关键的一步——tool 的输出不是临时变量，是会一直占着 token 的消息。你看 22 个条目就占了 5%。如果是 Read 一个 500 行的文件，就可能占 8-12%。

这就解释了为什么上下文窗口治理是核心问题——ReAct 的本质决定了窗口必然被工具结果撑满。
</note>

<note slide="ReAct · next round">
第四帧——第二轮 ReAct 开始。Agent 看完目录列表，决定深入 app.rs。它注意到这是入口文件。

这里 Agent 用的是 Read 工具而不是 Bash cat。为什么？Read 工具返回带行号的结构化输出，Agent 后面想做 Edit 替换的时候能精确定位。另外 Read 有 limit 参数能控量——不读全文。

结果回灌之后，窗口跳到 24%。两轮工具调用就用掉接近四分之一，这就是 ReAct 的真实压力。
</note>

<note slide="ReAct · summary">
最后一帧——Agent 把三轮工具调用的结果拼起来，给出真正回答用户问题的总结。

注意几个数字：3 轮工具调用、窗口最终 38%。但用户的原始问题只有一行字。中间几千个 token 几乎全是中间产物——目录列表、源码片段、grep 匹配——这些信息在结论给出之后价值就大幅下降。

如果接下来用户继续问问题，这些中间产物就会一直占着窗口。这就是下一节要解决的问题——上下文窗口治理。
</note>

<note slide="§02 Divider">
</note>

<note slide="Context Window Challenge">
第二章。上一章的 ReAct 循环已经直观展示了问题——每轮工具调用都在消耗上下文窗口，工具结果是主要膨胀源，且信息价值衰减极快。窗口被填满后，Agent 就会幻觉、重复操作、忘记目标。

我们的治理方案分两条路径。第一条：分级压缩——micro 级别快速把旧的工具结果替换成占位符，auto 级别调用 LLM 做深度摘要。两条路径都配豁免机制，保护关键信息不被压缩掉。第二条：优先级窗口——不是固定取最近 N 条消息，而是给消息打价值分，按 3:6:1 的比例填充窗口。实测可以稳定运行 2000+ 轮。

两条路径共享两个机制：豁免机制保护高价值消息，动态 System Prompt 让 Agent 感知当前正在发生的持续性事件。
</note>

<note slide="Compaction System">
分级压缩的第一级是 micro 压缩。它做的事情很简单——把相对久远的工具调用结果替换成占位符。比如原来 Read 了一个 500 行的文件占 8K token，micro 压缩后变成"[Read src/main.rs · 500 lines]"这样一个占位符，只占几十个 token。速度快，适合窗口快满时紧急释放空间。

第二级是 auto 压缩。它调用 LLM 把整个对话历史总结成一段精简摘要，然后清空窗口，把摘要写回去。成本高但压缩彻底——窗口从 97% 可以回到 30% 多。适合 Agent 已经跑了很多轮、需要一次大扫除的场景。

关键设计是豁免机制。无论是 micro 还是 auto，高价值消息都不参与压缩：用户消息是需求源头、Agent 的纯文本回复是决策记录、TodoWrite/Plan/Skill 是方向性信息、SubAgent 最终结果是调研结论。这些一旦压缩就会导致 Agent 跑偏。

反过来，Bash、Edit、Read、Write 这些操作性工具的结果价值衰减极快——第 3 轮的 Read 结果，到第 20 轮时对决策几乎没价值了。所以它们不豁免，优先被压缩。
</note>

<note slide="Priority Funnel">
优先级窗口要解决的问题是：固定取最近 N 条消息，方向性信息可能在很早的位置被新消息挤出。比如用户在第 1 轮说"我要改认证模块"，到第 50 轮时这条消息可能已经不在窗口里了，Agent 就忘了自己在干什么。

我们的方案是把上下文当作推荐系统——给消息打价值分，优先填高价值的。高价值消息包括：用户消息（需求源头）、TodoWrite（当前任务追踪）、Clean 即 Plan clear-and-approve（执行方案）、Skill 加载（能力定义）、Agent 纯文本回复（决策记录）。

具体填充按 3:6:1 比例——30% 给高价值方向性信息，60% 给中等价值的近期上下文，10% 给低价值的执行细节。这样保证了 Agent 在长程运行时始终能看到方向性信息。

实测结果：可以稳定运行 2000+ 轮消息的持续任务，Agent 方向性保持稳定。2000+ 还只是跑到那里的数字，理论上还能继续。
</note>

<note slide="Dynamic System Prompt">
动态 System Prompt 是上下文治理的第三层保障。System Prompt 的尾部会根据当前状态动态追加信息——后台任务在跑、有 Teammate 存在、TODO 很久没更新了、有交互式 Session 在进行。即使这些事件的原始上下文已经被压缩掉了，Agent 仍然能感知到它们。

下一页会展示三重保障组合后的实测效果。
</note>

<note slide="Long Running Proof">
这一页是整个第二章的成果总结。三重保障——分级压缩 + 优先级窗口 + 动态 System Prompt——组合起来的效果，右边这张截图就是证明。

这是一个真实的长程运行任务，2000+ 轮消息。可以看到上下文窗口始终维持在健康水平，没有冲到危险区，Agent 的方向性也没有丢失。中间有多次 micro 和 auto 压缩触发，但每次都回到健康水平继续运行。

再加上指数退避处理 API 故障，整个系统足够鲁棒。你可以睡前审批 Plan，让 Agent 自己跑，早上起来收到一个可运行的原型。Agent 不会因为上下文爆满而幻觉或偷工减料。
</note>

<note slide="§03 Divider">
</note>

<note slide="Tool System">
第三部分讲工具体系。设计哲学是渐进增强：从最基础的工具开始，每个新工具解决一个具体问题。

最小 Agent 只需要 4 个工具：Read、Write、Bash、LoadSkill。理论上能工作了——能读文件、写文件、执行命令。但实际上远远不够。

面对未知项目，Agent 只能 Bash grep，匹配质量不稳定；用户需求模糊，Agent 盲目执行容易漂移；LLM 没想清楚就开始改，改一处忘一处；长任务跑着跑着忘记目标。每个增强工具都是为了解决一个这样的问题。

右边三层架构图，基础层就是最小 Agent，增强层每个工具下方是一句话动机。接下来的页面展开讲 Plan、Edit、执行三态、TodoWrite 这些关键工具。
</note>

<note slide="Plan · Edit · approve">
这一页讲三个紧密相关的机制：Plan、审批、Edit。

Plan 解决 LLM 边想边改的问题。进入 Plan 模式后工具集被限制为只读，Agent 只能调研不能改代码。调研完后制定自包含执行计划，用户审批。关键操作是清空上下文——把几十 K 的调研结果全丢掉，只留 plan.md。

审批三选项：① 批准 ② 批准并清空上下文（推荐）③ 驳回。推荐选项 ②，因为调研阶段的信息对执行阶段是噪音——窗口从 65% 回到 12%。

Edit 解决 Write 的粗粒度问题。Write 要全文覆写，Edit 只需要指定"把这段旧字符串替换成这段新字符串"，精确、高效。右下方对比图：Write 是 500 行全发，Edit 只发匹配和替换的两小段。
</note>

<note slide="Plan · execute">
第三阶段——关键操作。窗口被清空。原本 65% 的调研产物全部丢弃。新的上下文里只有一份 plan.md，1.2K token 左右。窗口占用回到 12%。

Agent 重新启动，工具集恢复完整。它对照 plan 一步一步做：step 1 创建 ui_state.rs、step 2 创建 chat_state.rs……

这个机制的代价是 Agent 忘掉了调研细节。但收益更大——执行阶段窗口干净，不被中间产物干扰，错误率显著下降。这就是 clear and approve 的本质。
</note>

<note slide="执行三态 · concept">
这一页讲 Agent 的命令执行模型。先说问题背景——我在生成业务软件时遇到一个真实的卡死案例。

Agent 执行了 npm create vite@latest my-app -- --template react-ts。这个命令是用来初始化一个 React + TypeScript 项目的。但命令执行后久久没有返回，窗口占用不断上涨。

排查发现这个命令有交互式设计——它会在安装前提示用户确认，要求输入 Enter。但 Agent 用的是同步 Bash 调用，进程在等 stdin，Agent 在等 stdout，双方互相等待形成死锁。

这类交互式命令在包管理器中非常常见——npm init、pip install 会问 y/n，Homebrew 会问是否继续。Agent 必须能识别并正确处理。

解决方案是三态执行模型：同步 Bash 处理短命令、BackgroundTask 处理长命令、Session 处理需要 stdin 的交互式命令。入口统一都是 Bash 工具，运行时自动识别命令特性落到对的形态。

右图三条泳道是横轴时间。重点看那条虚线箭头——同步 Bash 跑过 30 秒还没结束，运行时不会 kill 进程，而是把这个已经在跑的进程连同 reader 线程一起移交给 BackgroundManager，返回 task_id。

但这又引入新问题：命令是慢、还是在等用户输入？外面看都是"没输出"。所以升级前我加了一道静默检测——如果最近 N 秒一点输出都没有，那大概率不是慢，是在等 stdin。这时候不该升级，而是 kill 掉，提示 Agent 加上 -y 之类的非交互标志重跑，或者改用 interactive:true 走 Session。

下一页用真实代码展示这两个关键机制的实现。
</note>

<note slide="执行三态 · impl">
这一页用真实代码展示两个关键机制的实现。

adopt_process 自动升级。代码里的核心逻辑是：超过 30s 阈值后，先做静默检测——如果最近 N 秒一点输出都没有，说明命令不是在跑，而是在等人输入，这时候 kill 掉给提示。如果不是静默，就调 adopt_process 把当前进程移交给 BackgroundManager，不 kill，返回 task_id。后台有个监控线程等 reader 写完后调 complete_task。

PTY Session。用 portable_pty 库分配伪终端。关键三行：clone reader 给 Session 读输出、take writer 给 Session 写输入、set_pty_writer 注册到 BgTask。之后 Session 工具的三个操作都是直接操作 PTY 句柄：stdin 写入、stdout 读 buffer、quit 就是 drop writer 让进程收到 SIGHUP。
</note>

<note slide="System Reminder">
这一页讲 system-reminder——一个让 Agent 在长任务中保持状态感知的机制。

核心问题：Agent 每轮推理只能看到上下文窗口内的内容。一旦信息被压缩摘要了、或者滚出窗口了，Agent 就彻底失去感知。比如后台跑了一个 cargo build，跑完的时候 Agent 已经在干别的了，它不知道 build 完成了、结果是什么。再比如待办清单写了 4 件事，Agent 一头扎进第 2 件，忘了后面还有 3 件。

解决方案是 system-reminder。它不是静态的系统提示词，而是每轮 LLM 请求前，由 PreLlmRequest Hook 动态构建、以 User 角色注入的消息。

有两层机制。第一层是 system_prompt 里的占位符替换——system_prompt 模板里有 {{.background_tasks}}、{{.session_state}}、{{.teammates}} 这些占位符。每轮请求前，Hook 把它们替换成实时状态：哪些后台任务在跑、工具注册了什么、Teammate 在干什么。这确保 Agent 每轮都能看到当前环境的完整快照。

第二层是 system-reminder 消息注入。当有突发事件时——后台任务完成、Teammate 发来新消息、待办清单太久没更新——运行时会构建一条 system-reminder 消息，以 User 角色插入上下文。

右边上面展示的是第一层——system_prompt 中的占位符替换。下面展示的是第二层——一个后台任务完成的通知，Agent 读到就知道 build 跑完了、结果如何。

这个设计的关键是软提醒，不强打断。Agent 不是被中断的，而是在下一轮推理时自然看到这些信息。如果它正在处理更紧急的事，可以选择暂时忽略。
</note>

<note slide="§04 Divider">
</note>

<note slide="Multi-Agent Collaboration">
第四部分，多 Agent 协作。j-cli 支持两种协作模式。

Teammate 是对等的 Agent 实例——每个都有独立的上下文窗口和工具集。它们通过 SendMessage 广播消息，用 @提及 定向通信。支持 WorkDone 声明完成，被 @self 提到可以重新激活。右上的架构图展示了三通道设计：broadcast_inbox 是每个 Teammate 的收件箱，context_messages 是注入 Main Agent LLM 上下文的通道，display_messages 是 TUI 渲染通道。

SubAgent 完全不同——它是临时雇员，在独立窗口里执行任务，不能收发消息，执行完只回灌一段摘要。三份调研合计 ~71K tokens 在 SubAgent 独立窗口里消耗，主 Agent 只收到三段 1.5K 的摘要，窗口仅微增 4%。

还有一个有意思的设计——客观评估。Anthropic 研究指出模型倾向偏袒自己的工作，通过聊天室让其他 Agent 交叉评审，能拿到相对客观的反馈。
</note>

<note slide="Message Design &amp; Visibility">
这一页讲多 Agent 之间消息怎么流转、谁能看到什么。

首先是三通道架构。同一逻辑消息有两种物理表示：display 通道给 TUI 渲染，用 clean text；context 通道给 Main Agent 的 LLM 上下文，用 XML-wrapped 标签标出来源。这样 Main Agent 在推理时能知道"这段话是谁说的"。

这里只介绍了消息通信的设计。Agent Loop 长时序异步导致的重复消息问题，以及 SendMessage Gate 的解决方案，在下一个实验页（轮流报数）中通过实验→问题→机制→结果的顺序详述。

右边的可见性表格展示了三者的信息不对称设计：Main 能看到其他 Agent 的文本和工具调用名；Teammate 只能看到广播文本；SubAgent 完全看不到其他 Agent。工具结果一律不推入其他 Agent 的上下文——因为这些执行中间过程价值低但 token 占用高。
</note>

<note slide="Exp1a · 轮流报数 — 问题">
这个实验本来是为了验证消息传递与唤醒。两个 Teammate 轮流报递增数字，仅通过 SendMessage 沟通。

但实验中暴露了核心问题——过期快照导致重复消息。1 号 Agent 进入循环时，信箱快照里 2 号的 "2" 还没到达，1 号只看到 [1]，就催促 2 号报 2。2 号收到催促后重复报 2——对外不一致。

根本原因是 Agent Loop 长时序异步：发完 SendMessage 不原地等确认，下一轮基于过期快照决策。下一页讲怎么解决。
</note>

<note slide="Exp1b · SendMessage Gate">
解决方案——二阶段提交 Gate。Agent 调 SendMessage 前，先检查 broadcast_inbox 有没有未读消息。如果有，hold 住，注入 system_reminder 让 Agent 重新决策。信箱从并行切换到串行化模式，Agent 可以看到最新消息后 commit 或重新决策。最多重试 2 次。

右边时序图展示了解决后的流程——1 号欲报 3 时被 Gate 拦截，看到 [1,2]，确认报 3，对外一致。

实测结果：10 个数字全部正确报出，顺序错误 0，唤醒失败 0，人工干预 0。SendMessage 调用 16 次（含 Gate 触发的二次确认），证明机制有效。
</note>

<note slide="Exp2 · 续写接龙">
实验二把验证重点从"能按序发言"推进到"能否沿用同一组事实"。

实验结果论文表 7-11：13 轮全部完成，偏题 0 次，人工干预 0 次——说明共享上下文确实让 Agent 能读到前文设定。

有一次重复——「反转之王」这个角色被 Main 催促后重发了同样的内容。这说明协作协议可以自主完成，但仍需要消息去重和过期约束。对应到工程场景就是：多 Agent 可以共享需求和接口事实，但任务边界必须明确。
</note>

<note slide="Exp3 · 协作开发">
实验三是真实工程场景了——电商网页项目追加 45 项验收清单。Developer 写代码，Tester 验收。

这里有个关键设计动机参考了 Anthropic 那篇 harness-design 文章——模型容易对自己的工作过度满意，让另一个 Agent 来评审才能拿到客观反馈。

结果论文表 7-13：第 1 轮 41 通过 4 失败，Developer 一次性修完那 4 项，第 2 轮 45/45 全过。需求完成率 100%、协同冲突 0、重复劳动 0。两次人工干预都是流程调度层面（提醒创建 Tester、提醒启动验收），不涉及代码或需求理解。

顺便提一句健壮性——LLM API 难免遇到限流、超时，我用指数退避自动处理瞬态故障（1s→2s→4s），不打断协作流程。
</note>

<note slide="SubAgent Delegation">
SubAgent 和 Teammate 不一样——Teammate 是平等的协作者，SubAgent 是临时雇用的调研员。

核心思想是上下文隔离。主 Agent 想要了解一个未知模块，如果自己去 Read 十几个文件，主窗口会被瞬间填满几十 K token。这些原始内容里 90% 都是噪音，主 Agent 真正需要的只是一段结论。

隔离体现在三个层面：上下文隔离——独立消息窗口，不继承父 Agent 历史，以 prompt 为唯一起点；工具隔离——独立 ToolRegistry，排除 Teammate 和 Agent 工具防止递归，可选 worktree 创建独立 git 工作树；结果回灌——执行完毕后只推精简摘要，工具结果不推入主 Agent 上下文。

右边这张图——三个 SubAgent 并行调研，每个用掉 20-30K tokens 的独立窗口。但主 Agent 收到的只是三段总计 1.5K 的摘要。主窗口仅微增 4%，但获得了三个模块的完整理解。

这就是"以较低的上下文成本了解调研目标"——也是 SubAgent 这个抽象最大的价值。
</note>

<note slide="§05 Divider">
</note>

<note slide="Template Engine · problem">
第五章讲模板代码生成引擎。先说问题。

业务系统里有大量高度模板化的代码——PO/DTO/VO/DAO、CRUD 方法、列表查询。它们占比高，但业务含量有限。让 LLM 逐字生成会怎样？

51 张表的 DAO 生成实测：LLM 漏了 11 个字段、4 个索引方法，烧了 24M token、跑了 53 分钟，触发了 67 次压缩。写到第 30 张表时，Agent 已经忘了第 5 张表的唯一索引。

右边是两条路径的对比。上面是 LLM 直接生成——字段漂移、token 高、慢。下面是模板化生成——100% 字段对齐、token 只要 1/50、27 倍加速。关键差异：LLM 是概率模型，每一步都有漂移风险；模板是确定性的，schema 是唯一事实源。

但问题来了——为什么我们确信模板化可以覆盖这些代码？下一页讲核心观察。
</note>

<note slide="Template Engine · observation">
上一页讲了问题——让 LLM 直写模板代码代价很高。这一页回答：为什么我们确信模板化可行？

大家看右边这张图。左边是一份 SQL 的 CREATE TABLE 语句——有字段定义、主键、唯一索引、普通索引。右边是对应生成的 Go 代码：上面是 Model struct（PO），下面是 DAO 层的查询方法。

你会发现——从左边到右边，几乎不需要任何"创造性决策"。字段名直接映射，类型有固定对应关系，索引类型决定了该生成什么查询方法。业务逻辑对这些代码的可操作独立发挥空间非常有限。

换句话说，DDL 是数据层代码的充分输入。既然如此，这些代码不需要 LLM 来逐字推理。我们把它交给确定性生成器 jen，封装为 Skill 按需加载。

下一页看 jen 的架构设计。
</note>

<note slide="jen Architecture">
先讲设计初衷——为什么要做 jen。复杂业务软件里有很多"机械代码"，PO/DTO/VO/DAO 这些，业务含量有限，但跟字段命名、类型映射强相关。LLM 逐字生成容易字段漂移、前后端不一致。

核心思路是互补而非替代——LLM 擅长需求理解和流程调度，模板生成器擅长确定性输出。Agent 只负责准备 schema 和验收结果，不逐字写模板代码。

自研 parser 的原因是需要精确提取索引语义——主键、唯一索引、普通索引分别能派生什么查询方法。通用 parser 的 AST 里这些信息是隐式的，必须自己走一遍。

最终产物的关键是按索引派生方法。这是 LLM 直写最容易漏的环节——51 张表实测 26 个索引 LLM 漏了 4 个。下一页看实测数据。
</note>

<note slide="jen Benchmark">
这页是论文 §7.4 的实测数据。先说验证方式——51 张表，同一份 schema，局部对照。

关键发现：不只是更快，而是过程成本和字段精度有本质差异。LLM 直写需要反复在字段对齐、DAO 方法补全、编译修复之间切换——长上下文维护本身成为主要负担。67 次 compact 说明 51 张表已把上下文撑爆。

11 个字段遗漏和 4 个索引方法缺失不是随机错误，而是长链路推理的系统性风险——写到第 30 张表时，Agent 已经忘了第 5 张表有哪些唯一索引。jen 以 schema 为唯一事实源，从根本上消除这类漂移。

结论：确定性生成器的价值不是替代 Agent 全部能力，而是把规则稳定、易字段漂移的部分剥离出 LLM 链路。
</note>

<note slide="webapp-gen Flow">
webapp-gen 是 §5 的第二个 Skill，目标是生成完整全栈 Web 应用。

它不是"AI 一次性写完一个 App"，而是一条 11 步固化工作流。给大家逐步看一下。

Step 0 强制前置：git clone proj_template，先有可跑的最小骨架——React + TS + Tailwind + Go + Gin + GORM。结构完整性由模板兜底。

Step 1–3 是文档驱动：先写需求，再定 API 契约（统一信封、错误码、分页、鉴权），再做前端设计。API 文档是前后端唯一契约——文档没定义的字段不允许出现在代码里。

Step 4–5 是关键的反馈闭环：先用 mock 数据生成可视原型，让用户在还没写后端代码的时候就校准预期。这一步迭代多少轮都比"全做完才发现不对"便宜。论文实测案例迭代了 13 轮原型反馈。

Step 6 用 jen Skill：写完 SQL schema 后调 @skill:jen，把 PO/DTO/VO/DAO 一次生成。这就是上一页对比里 jen 的实战位置。

Step 7–10 后端编码→黑盒测试→前端编码→容器化验收。其中黑盒测试有个特别设计——三环境矩阵：本地 + podman 单容器 + 全栈 podman compose，三个环境同一份脚本都要过，避免"本地好好的容器里炸了"。

整个工作流的核心思想是：把 AI 不擅长的部分交给模板，AI 只做需求理解、契约定义、增量集成。最终产物在右下角的开源仓库可以直接看。
</note>

<note slide="§06 Divider">
</note>

<note slide="Repo Gallery">
讲完了所有设计，给大家看一下整个毕设的产出——三个 GitHub 仓库。

第一个是主项目 jcli。Rust 写的，大概 24K 行代码。所有今天讲到的内容——chat 模块、工具体系、Skill 系统、Hook 系统、多 Agent 协作——都在这一个仓库里。

第二个是 model_infrax，里面装的是 jen 代码生成器。Go 写的 SQL parser + 模板引擎。它本身是独立工具，但被封装成 skill 集成进 j-cli。

第三个是 gradiation_artifact_demo——用 j-cli 配合 webapp-gen 和 jen 两个 skill 真实生成出来的电商 Web App。下一页展示实际界面。三环境矩阵验收全部通过，算是整套工具链的端到端验证。

三个仓库都是开源的，MIT 协议。欢迎大家访问、star、提 issue 或者 PR。
</note>

<note slide="Demo Preview">
这是用 j-cli 配合 webapp-gen 和 jen 两个 skill 真实生成出来的电商 Web App——gradiation_artifact_demo。

六张截图展示了完整的用户流程：商品列表、商品详情、购物车、订单创建、订单列表、订单详情。

技术栈是 React 前端 + Go 后端 + GORM ORM + Podman 容器化。三环境矩阵验收全部通过——本地开发环境、测试环境、生产环境。

这算是整套工具链的端到端验证——从需求描述到完整可运行的 Web App，全程由 j-cli 驱动。
</note>

<note slide="Summary">
做个总结。这个毕设的核心贡献有四块。

① 上下文窗口治理——分级窗口 + 多级压缩 + 豁免高价值消息 + 动态 System Prompt。把上下文当推荐系统优化：决策多放、中间过程少放。

② 面向场景强化的工具——每个工具都是为解决具体问题而引入：Grep 主动构建上下文、Ask 防漂移、Plan 调研后清空上下文重新执行、Edit 精确替换、执行三态自动流转处理交互式命令、system-reminder 状态感知注入、Hook 14 事件守规范。

③ 多 Agent 协作试验——三个实验从简到难：轮流报数验证消息层、续写接龙验证共享上下文、协作开发验证工程闭环。三组数字都在论文表里——100%、13/13、45/45。SendMessage Gate 是为解决长时序重复消息设计的关键机制。

④ 模板代码生成引擎——jen 把 51 张表的实测从 24M token 降到 48 万、53 分钟降到 2 分钟、字段一致性从 95.7% 升到 100%。webapp-gen 把全栈生成固化为 11 步工作流。两者都用 Skill 按需加载，不污染上下文。
</note>

<note slide="Thanks">
好，今天就分享到这里。做一个简单的收尾。

回到最开始的那张对比图——Agent 的核心价值，就是通过上下文治理和工具调用，让 AI 自己成为物理世界（代码、终端、文件系统）和逻辑世界（LLM 思考空间）的桥梁，把人从这个循环中抽离出来。

我的项目叫 j-cli，所有代码都在 GitHub 上开源。jen 代码生成工具在 model_infrax 仓库，毕设相关制品在 gradiation_artifact_demo 仓库。欢迎各位老师同学批评指正。

谢谢大家！有问题可以开始提问。
</note>

</speaker-notes>
`;
