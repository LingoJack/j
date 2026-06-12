# 简化 jstudio 设置面板

## 背景
当前 DocumentList.tsx 底部设置面板（popover）包含大量功能：恢复初始数据、清空本地文档、JSON 备份/导入等。用户要求简化为只保留主题切换。

## 改动范围

### 1. DocumentList.tsx — 设置面板精简
- 删除整个 Settings Popover（`showSettingsPopover` 及其内容）
- 删除底部"库设置"齿轮按钮
- 保留底部主题切换按钮（太阳/月亮图标），作为唯一的设置项
- 清理不再需要的 state：`showSettingsPopover`、`importText`、`showImporter`
- 清理不再需要的 props：`onRestoreDefaults`、`onClearAll`、`onImportData`
- 清理不再需要的 import：`Settings`、`RefreshCw`、`Package`

### 2. App.tsx — 清理无用回调
- 删除 `handleRestoreDefaults`、`handleClearAll`、`handleImportData` 三个函数
- 删除传递给 DocumentList 的三个 props：`onRestoreDefaults`、`onClearAll`、`onImportData`

### 3. BlockItem.tsx — 清理无用 import
- 删除 `Settings` import（如果仅用于已删除的设置面板）

## 底栏最终布局
```
[底栏] ─── 左对齐：主题切换按钮（齿轮图标 + 太阳/月亮图标合一）─── 右对齐留空 ───
```

改为：底栏只有一个主题切换按钮（点击即可切换），居左显示。
