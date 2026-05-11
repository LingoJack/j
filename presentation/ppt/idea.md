主要的演说 idea
要让过程通顺起来，就要从局限性触发，在途中遇到了什么问题，怎么解决的，为什么这样能解决
1. 消息分级窗口和滚动窗口主要解决了什么问题？
2. edit 工具强化的目的是什么，又是为解决什么问题
3. plan 工具又是为了解决什么问题？我的核心观点是，LLM 有时候没有想清楚就开始做，这样导致的问题就是需要变更的地方改不全，且调研过程占用了大量上下文（因此还提供了一个 clear and approval 的选项）
4. 压缩 compact 系统的设计为什么要豁免一些工具？因为这些是关键的，例如 Read、Edit、Bash 这些就不豁免，因为 Agent 调用这个信息的价值随着轮次进行下降很快；而采用多级压缩 micro, auto 就是兼顾了速度和准确率，让整个 context window 可以处在健康低幻觉的状态，大大提高了 agent 的长期运行的能力。高价值的消息包括：用户消息、Agent 语言消息、System 提醒消息（TODO、Teammate）、Plan、Skill、TODOWrite、TODOREAD、SubAgent
