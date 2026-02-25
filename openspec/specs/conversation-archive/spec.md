## ADDED Requirements

### Requirement: 归档当前对话
系统 SHALL 允许用户将当前对话归档保存，归档后当前会话被清空。

#### Scenario: 使用默认名称归档对话
- **WHEN** 用户按下 Ctrl+L 且当前会话有消息
- **THEN** 系统提示归档确认，默认名称为当前日期（YYYY-MM-DD格式）
- **AND** 用户确认后，对话保存到归档文件
- **AND** 当前会话消息被清空
- **AND** 显示成功提示 "对话已归档: <名称>"

#### Scenario: 使用自定义名称归档对话
- **WHEN** 用户按下 Ctrl+L 且当前会话有消息
- **AND** 用户输入自定义名称
- **THEN** 对话以自定义名称保存到归档文件
- **AND** 当前会话消息被清空

#### Scenario: 取消归档操作
- **WHEN** 用户按下 Ctrl+L 且当前会话有消息
- **AND** 用户按下 Esc 取消
- **THEN** 不执行归档操作，当前会话保持不变

#### Scenario: 空会话无法归档
- **WHEN** 用户按下 Ctrl+L 且当前会话无消息
- **THEN** 系统显示提示 "当前对话为空，无法归档"

### Requirement: 归档数据持久化
系统 SHALL 将归档数据以 JSON 格式保存到本地文件系统。

#### Scenario: 创建归档文件
- **WHEN** 用户归档对话
- **THEN** 系统在 ~/.j/chat/archives/ 目录创建 JSON 文件
- **AND** 文件名格式为 <名称>.json
- **AND** 文件内容包含 name、created_at、messages 字段

#### Scenario: 归档目录自动创建
- **WHEN** 应用启动时归档目录不存在
- **THEN** 系统自动创建 ~/.j/chat/archives/ 目录

### Requirement: 归档命名规范
系统 SHALL 对归档名称进行校验，确保文件名合法性。

#### Scenario: 默认名称格式
- **WHEN** 用户使用默认名称归档
- **THEN** 默认名称格式为 archive-YYYY-MM-DD（如 archive-2026-02-25）

#### Scenario: 默认名称冲突自动处理
- **WHEN** 用户使用默认名称归档
- **AND** archive-2026-02-25 归档已存在
- **THEN** 系统自动使用 archive-2026-02-25(1) 作为名称
- **AND** 如果 archive-2026-02-25(1) 也存在，使用 archive-2026-02-25(2)，以此类推

#### Scenario: 自定义名称校验
- **WHEN** 用户输入自定义名称
- **AND** 名称包含非法字符（/ \ : * ? " < > |）
- **THEN** 系统显示错误提示 "名称包含非法字符"
- **AND** 允许用户重新输入

#### Scenario: 名称长度限制
- **WHEN** 用户输入自定义名称
- **AND** 名称超过 50 字符
- **THEN** 系统显示错误提示 "名称过长，最多 50 字符"
- **AND** 允许用户重新输入

#### Scenario: 名称冲突处理
- **WHEN** 用户输入的名称与现有归档同名
- **THEN** 系统提示 "归档已存在，是否覆盖？"
- **AND** 用户确认后覆盖原归档文件
- **OR** 用户取消后允许重新输入

### Requirement: 归档文件格式
系统 SHALL 使用标准 JSON 格式存储归档数据。

#### Scenario: 归档文件结构
- **WHEN** 系统保存归档文件
- **THEN** JSON 结构包含：
  - name: 归档名称（字符串）
  - created_at: 创建时间（ISO 8601 格式）
  - messages: 消息数组，每条消息包含 role 和 content

#### Scenario: 消息格式
- **WHEN** 系统保存消息到归档
- **THEN** 每条消息包含 role 字段（"user" 或 "assistant"）
- **AND** 每条消息包含 content 字段（消息内容字符串）

### Requirement: 归档列表管理
系统 SHALL 提供归档列表查看功能。

#### Scenario: 查看归档列表
- **WHEN** 用户按下 Ctrl+R 进入还原模式
- **THEN** 系统显示所有归档列表
- **AND** 列表按创建时间倒序排列
- **AND** 每个归档显示名称和消息数量

#### Scenario: 无归档时提示
- **WHEN** 用户按下 Ctrl+R
- **AND** 不存在任何归档文件
- **THEN** 系统显示 "暂无归档对话"

### Requirement: 归档删除功能
系统 SHALL 允许用户删除不需要的归档。

#### Scenario: 在归档列表中删除
- **WHEN** 用户在归档列表中选中某个归档
- **AND** 用户按下 d 键
- **THEN** 系统提示确认删除
- **AND** 用户确认后删除归档文件
- **AND** 显示 "归档已删除: <名称>"
