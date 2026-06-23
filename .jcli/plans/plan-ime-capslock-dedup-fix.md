# 修复中文输入法 + CapsLock 切换导致重复/空格字符

## 问题背景

在 jstudio 的 BlockEditor（TipTap/ProseMirror 编辑器）中，使用中文输入法时按 CapsLock 切换中英文，会出现重复字符或多余空格。

**根因分析（macOS WebKit + 中文 IME）：**

用户正在输入拼音（composition 进行中），此时按下 CapsLock 切换到英文模式，事件序列为：

1. `keydown`（key: "CapsLock", `isComposing: true`）— OS 层已切换输入法
2. `compositionend` — IME 提交当前候选项
3. `beforeinput`（insertText）— 提交的文本写入 DOM
4. **多余的** `beforeinput`（insertText）— 部分中文 IME（搜狗/系统拼音）在模式切换边界会重复提交或泄漏一个空格

ProseMirror 自身有 composition 追踪机制，但无法预期 CapsLock 这种「硬件级中断」打乱的时序，导致第 4 步的幽灵输入被当作正常文本插入。

## 方案设计

创建一个新的 TipTap Extension `ImeCapsLockFix`，作为 ProseMirror Plugin 拦截幽灵输入。

### 新增文件

`apps/jstudio/src/lib/extensions/imeCapsLockFix.ts`

### 核心逻辑

```
状态机:
  composing: boolean        — 是否在 composition 中
  lastCommittedText: string — 最近一次 compositionend 提交的文本
  lastCommitTime: number    — compositionend 时间戳
  capsLockInCompose: boolean— composition 期间按了 CapsLock

事件处理:
  compositionstart  → composing = true
  compositionend    → composing = false; 记录 data + 时间
  keydown(CapsLock) → 若 composing == true，置 capsLockInCompose = true + 时间戳
  beforeinput       → 若 capsLockInCompose 且 inputType 为 insertText:
                        - 文本 == lastCommittedText → preventDefault（去重）
                        - 文本为单个空格且距 commit < 200ms → preventDefault（去幽灵空格）
                      超时（>500ms）或正常输入后自动清除标志
```

### 实现要点

1. **通过 `Extension.create().addProseMirrorPlugins()`** 注册 Plugin
2. **Plugin 使用 `props.handleDOMEvents`** 挂载 `compositionstart`、`compositionend`、`keydown`、`beforeinput` 四个 DOM 事件监听器
3. **额外用 `props.handleTextInput`** 作为二级防线，捕获绕过 beforeinput 的文本插入（部分旧 WebKit 版本）
4. **所有状态保存在 Plugin `state`** 中（通过 `PluginKey`），避免全局变量；或使用闭包内的 `let` 变量（因为这是纯副作用拦截，不需要触发 re-render）

### 为什么用闭包而非 Plugin State

此插件是纯拦截型（preventDefault），不需要参与 ProseMirror 的 transaction/state 流转。使用 Plugin 闭包内的局部变量更简洁，且不会产生多余的 state diff。

## 集成

在 `BlockEditor.tsx` 的 `useEditor({ extensions: [...] })` 中添加：

```typescript
import { ImeCapsLockFix } from '../lib/extensions/imeCapsLockFix';
// ...
extensions: [
  // ... existing extensions ...
  ImeCapsLockFix,   // ← 新增
],
```

## 影响范围

- **仅影响 BlockEditor**（TipTap 编辑器），不影响标题 `<input>`（标题是原生 input，其 IME 行为由浏览器原生处理，不存在 ProseMirror 的双重提交问题）
- **不影响正常打字**：只在「composition 期间按了 CapsLock」这个特定场景下激活
- **不影响快捷键**：CapsLock 不是任何快捷键的组成部分
- **read-only 模式**：无影响（编辑器不可编辑，无 composition）

## 测试验证

1. `make dev-jstudio` 启动 Tauri dev
2. 在编辑器中使用系统拼音/搜狗输入法输入中文
3. 输入拼音过程中按 CapsLock 切换到英文
4. 验证：无重复字符、无多余空格、之前输入的中文正确保留
5. 验证：切换到英文后继续打字正常
6. 验证：正常中英文输入不受影响
