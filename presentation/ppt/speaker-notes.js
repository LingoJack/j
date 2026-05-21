// 演讲稿数据 · 10 分钟答辩版（已与 speaker-script.md 对齐）
// 同步规则：以 speaker-script.md 为唯一信息源；改稿后用脚本同步覆盖本文件。
var SPEAKER_NOTES_XML = `
<?xml version="1.0" encoding="utf-8"?>
<speaker-notes>

<note slide="Cover">
各位老师好，我是温俊杰。我的毕设题目是《基于多 Agent 协同的 LLM 驱动复杂业务软件自动化生成的研究与实现》。下面我用大约十分钟向各位老师汇报。
</note>

<note slide="项目">
这是本次毕设的三个产出。其中 jcli 是我自己用 Rust 实现的 Agent 工具，也是我整篇毕设里大部分想法的主要实验载体——后面要讲的上下文治理、工具强化、多 Agent 协作这些设计，都是在它上面做的；jen 是配套的模板代码生成引擎；最后是用这两者端到端生成出来的一个电商 Web demo。
</note>

<note slide="Agenda">
下面我会按背景、上下文治理、工具强化、多 Agent 协作、模板代码生成、最后总结这六个部分依次汇报。
</note>

<note slide="ChatBot vs Agent">
这个项目的出发点，是我自己在用 ChatBot 写代码到用 Agent 写代码这个过程里观察到的一个变化——连接物理世界和逻辑世界的载体在变。ChatBot 时代，是我手动收集代码、报错信息再粘贴回去；到 Agent 时代，这件事开始由程序通过协议自己承担。所以我整篇毕设的两条主线，也就落在两件事上：上下文怎么管、工具怎么用。
</note>

<note slide="Function Call">
Agent 之所以能"做事"，靠的是 ReAct 范式加 Function Calling 协议。LLM 自己只能输出文本，所以做法是把工具的 Schema 先注册进 System Prompt；LLM 在推理时输出一段 tool_calls JSON，运行时拿着 JSON 真正去执行，再把结果作为消息回灌。LLM 只产生意图，运行时代它行动。
</note>

<note slide="ReAct · concept">
ReAct 每一轮分四步：Thought 决策、Action 生成 tool call、Observation 工具执行回灌、Update 进入下一轮。这就是 Agent 的心脏。
</note>

<note slide="ReAct · user">
下面这五页我用一个真实任务把这个循环走一遍：用户问"总结 src/command/chat 的结构和优化点"。Agent 一共做了三轮工具调用——先 ls 列目录、再 Read 入口文件、再 Grep 关键定义——最后给出总结。但就这三轮，上下文从 6% 一路涨到了 38%，中间几千 token 几乎都是一次性的中间产物。这也就引出了下一章——为什么必须治理上下文。
</note>

<note slide="ReAct · action">
（连续翻过，不停顿。）
</note>

<note slide="ReAct · observation">
（连续翻过，不停顿。）
</note>

<note slide="ReAct · next round">
（连续翻过，不停顿。）
</note>

<note slide="ReAct · summary">
（连续翻过，不停顿。）
</note>

<note slide="§02 Divider">
（一带而过。）
</note>

<note slide="Context Window Challenge">
ReAct 的本质决定了窗口必然会被工具结果不断撑满；一旦撑满，Agent 就开始幻觉、重复操作、忘记目标。所以这里我做了三重保障：分级压缩、优先级窗口、动态状态感知。
</note>

<note slide="Compaction System">
压缩做了两级：micro 把比较久远的工具结果换成占位符，便宜、快，但只是局部腾空间；auto 调 LLM 把整段历史摘要后清空再写回，彻底但贵。这里有一个我比较看重的设计——豁免机制：用户消息、Plan、TodoWrite、SubAgent 结论这些方向性消息永远不参与压缩，只压缩 Read、Bash、Edit 这种衰减极快的执行结果。
</note>

<note slide="Priority Funnel">
固定取最近 N 条消息会有一个隐患——方向性消息容易被新消息挤出窗口。所以我换了个思路，把上下文当作推荐系统：给消息打价值分，按 3:6:1 的比例填窗——30% 给方向性消息、60% 给近期上下文、10% 给执行细节。
</note>

<note slide="Dynamic System Prompt">
Agent 还需要知道"窗口外正在发生什么"。我做了两层注入：System Prompt 里的占位符每轮替换为后台任务、队友状态等实时快照；突发事件则以 system-reminder 消息软插入，不强行打断，让 Agent 在下一轮自然看到。
</note>

<note slide="Long Running Proof">
这三层组合起来的效果，在我自己的实测里是——可以稳定跑到 2000 多轮消息，窗口始终维持在健康水平、方向也没有跑偏。再配合指数退避来兜底偶发的 API 故障，基本可以放着 Agent 自己跑、第二天来取结果。
</note>

<note slide="§03 Divider">
（一带而过。）
</note>

<note slide="Tool System">
工具这一部分我的思路是渐进增强。最小 Agent 其实只需要 4 个工具：Read、Write、Bash、LoadSkill；理论上够用，但实际上远远不够——会盲目执行、字段漂移、忘记目标。所以我为每一个具体问题加一个工具。下面挑三个最关键的展开。
</note>

<note slide="Plan · approve">
第一个问题是 LLM 边想边改、调研读 30 个文件就把窗口撑爆。我的做法是 Plan 模式：调研阶段只给只读工具、产出 plan.md，用户审批的时候顺便清空上下文——把几十 K 的调研产物丢掉、只保留 plan。窗口能从 65% 直接回到 12%。
</note>

<note slide="Plan · execute">
清空之后，Agent 重新启动、工具集恢复完整，就可以对照 plan 一步步实施。代价是 Agent 忘掉了调研细节，但收益是执行阶段窗口干净，错误率明显下降。
</note>

<note slide="Edit · replace">
Edit 工具我做了两个比较关键的设计。一是精确字符串匹配——基于 old_string 在文件中唯一匹配，匹配失败会给出诊断、匹配多次会提示加上下文。二是批量替换的凭证握手：replace_all 不直接执行，先返回预览和文件摘要凭证，AI 把凭证回传才执行；文件一变凭证就失效，AI 也没法预先伪造，因为凭证依赖它必须先看到的真实匹配结果。
</note>

<note slide="执行三态 · concept">
Bash 这部分我是从一个真实卡死案例倒推出来的。当时 Agent 跑 npm create vite，这个命令是交互式的，进程在等用户按回车、Agent 在等 stdout——双方互相等死锁。所以 Bash 必须有三态执行：短命令同步、长命令自动升级到后台、交互式命令走 PTY Session。关键设计是超时不直接 kill，而是先做静默检测——如果一直没有任何输出，那大概率就是在等用户输入，给 Agent 提示加 -y；如果还在出输出，就把进程移交后台、立即返回 task_id。
</note>

<note slide="执行三态 · impl">
（这页是上一页两个机制的真实代码，扫一眼即可，不停顿。）
</note>

<note slide="§04 Divider">
（一带而过。）
</note>

<note slide="Multi-Agent Collaboration">
多 Agent 我做了两种模式。Teammate 是对等协作者，每个有独立窗口和工具集，通过共享聊天室广播沟通。SubAgent 是临时调研员，独立窗口完全隔离，执行完只把一段精简摘要回灌给主 Agent——这样三个并行调研合计 70K token，主 Agent 只多用了 4%。
</note>

<note slide="Message Design &amp; Visibility">
为了支撑这两种模式，消息层我做了三通道架构和信息不对称的可见性：Main 看得到队友的文本和工具调用名，Teammate 之间只看广播文本，SubAgent 完全隔离——决策信息多放、执行细节少放。
</note>

<note slide="Exp1a · 轮流报数 — 问题">
第一个实验是两个 Teammate 轮流报数。本来只是想验证消息传递，但实验里反而暴露了一个我没预想到的问题——过期快照导致重复消息：Agent 发完消息不等确认就进入下一轮，看到的还是旧信箱。
</note>

<note slide="Exp1b · SendMessage Gate">
我的解决办法是 SendMessage Gate，二阶段提交：发消息之前先检查信箱有没有新消息，有就 hold 住、注入提醒、让 Agent 看到最新状态后再决策。修复后 10 个数字全部正确报出、人工干预 0 次。
</note>

<note slide="Exp2 · 续写接龙">
实验二把验证从"能按序发言"推进到"能否沿用同一组事实"——多个 Teammate 轮流续写故事、保持人物时间线一致。13 轮全部完成、偏题 0 次；只有 1 次内容重复，说明协作协议本身没问题，但仍需要消息去重。
</note>

<note slide="Exp3 · 协作开发">
实验三是真实工程场景：Developer + Tester 协作完成 45 项验收清单。这里有个我比较看重的参考——Anthropic 那篇论文指出模型对自己的工作有偏袒，让另一个 Agent 来评审才能拿到客观反馈。结果是第一轮 41 通过、4 项失败，Developer 一次性修完、第二轮 45/45 全过；需求完成率 100%、协同冲突 0。
</note>

<note slide="SubAgent Delegation">
SubAgent 的价值在于用很低的上下文成本去理解未知模块。它在窗口、工具、结果回灌三个层面都跟主 Agent 隔离，主 Agent 只拿到精简结论，不被原始噪音污染。
</note>

<note slide="§05 Divider">
（一带而过。）
</note>

<note slide="Template Engine · problem">
业务系统里有大量 PO/DTO/VO/DAO 这种高度模板化的代码。我做过实测：让 LLM 逐字生成 51 张表的 DAO，结果漏了 11 个字段、4 个索引方法，烧了 24M token、跑了 53 分钟、压缩触发了 67 次。这其实不是偶发错误，而是长链路推理的系统性风险。
</note>

<note slide="Template Engine · observation">
但仔细看这类代码会发现，它几乎不需要创造性决策：字段名直接映射、类型有固定对应、索引类型决定该有哪些查询方法。换句话说——DDL 已经是数据层代码的充分输入。既然如此，这部分就没必要让 LLM 来推理，交给确定性生成器更合适。
</note>

<note slide="jen Architecture">
jen 的原理就四步：输入 schema、自研 parser 解析索引语义、用 Go template 渲染、输出按主键/唯一索引/普通索引派生的原子方法集。确定性输出、零字段漂移。
</note>

<note slide="jen Benchmark">
同一份 schema 做对照实测，jen 把 token 降到了 1/50、耗时降到 1/27、字段一致性 100%。这里我想强调的是——引入它不是为了替代 Agent，而是把规则稳定、容易漂移的部分从 LLM 链路里剥离出来，让模型可以专注在业务逻辑和契约定义上。
</note>

<note slide="webapp-gen Flow">
把 jen 嵌进一个更大的流程，就是 webapp-gen——我固化的一条 11 步流水线：模板脚手架前置 → 文档驱动定契约 → mock 原型反馈闭环 → jen 生成数据层 → 后端编码 → 三环境矩阵容器化验收。背后的思路就一句话：AI 不擅长的交给模板，AI 只做需求理解、契约定义和集成验收。
</note>

<note slide="§06 Divider">
（一带而过。）
</note>

<note slide="Repo Gallery">
毕设产出最终是三个开源仓库：主项目 jcli，Rust 写的，大约 5.2 万行；模板引擎 model_infrax，也就是 jen；以及端到端验证制品 gradiation_artifact_demo。
</note>

<note slide="Demo Preview">
这是用 jcli 配合 webapp-gen 和 jen 真实生成出来的一个电商 Web 应用。技术栈是 React + Go + GORM + Podman，三环境验收都跑通了。
</note>

<note slide="Summary">
最后简单收一下。我做的四项工作之间是有递进关系的：上下文治理 + 工具强化保障了单 Agent 的长时运行；在此之上，多 Agent 协作提供了消息同步和上下文隔离；再在此之上，模板代码生成用确定性输出补上了 LLM 的短板。
</note>

<note slide="Thanks">
回到最开始的那张图——我理解的 Agent 的核心价值，就是通过上下文治理和工具调用，让程序自己成为物理世界和逻辑世界之间的桥梁。三个仓库都已经开源，欢迎各位老师批评指正。谢谢！
</note>

</speaker-notes>
`;
