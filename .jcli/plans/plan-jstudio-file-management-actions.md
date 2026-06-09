# jstudio 文件管理增强计划

## 背景

当前 `apps/jstudio/src/Reader.tsx` 与 `FileTree.tsx` 只支持：

- 新建文件
- 新建文件夹
- 打开文件
- 目录展开/折叠

缺少常见文件管理动作：

- 删除文件/目录
- 重命名文件/目录
- 在访达/Finder 中打开
- 对已打开 tab 的路径同步更新/关闭处理

## 现状耦合点

### 前端

- `Reader.tsx`
  - 管理 tabs、activeTab、dirty 状态、source/doc/image refs。
  - 目前只向 `FileTree` 传入：
    - `onOpen`
    - `onCreateFile`
    - `onCreateFolder`

- `FileTree.tsx`
  - 维护目录节点缓存 `nodes`。
  - 右键菜单当前只有「新建文件」「新建文件夹」。
  - `ContextMenuState` 当前只保存 `x/y/dir`，没有选中的 entry。
  - `PromptDialog` 可复用为「重命名」输入框。

- `api.ts`
  - 当前只有 create/list/read/save/quit 等 Tauri invoke 封装。

### 后端

- `src-tauri/src/lib.rs`
  - 当前只有 `create_file` / `create_dir`。
  - 需要新增 Tauri commands：
    - `rename_path`
    - `delete_path`
    - `show_in_folder`
  - 需要安全校验：路径必须存在、名称不能为空且不能包含路径分隔符、目标不能覆盖已有文件。

## 目标能力

### 文件树右键菜单

针对目录空白处/目录节点：

- 新建文件
- 新建文件夹
- 重命名
- 删除
- 在访达中打开

针对文件节点：

- 打开
- 重命名
- 删除
- 在访达中显示
- 复制路径（可选，如实现成本低则一起做）

### 删除行为

- 删除前用 `window.confirm` 或新增轻量 confirm dialog 二次确认。
- 文件使用 `std::fs::remove_file`。
- 目录使用 `std::fs::remove_dir_all`。
- 删除成功后刷新父目录。
- 如果删除的是已打开 tab：
  - 关闭该 tab；
  - 清理 `sourcesRef/docsRef/imagesRef/originalSourcesRef`；
  - 如果删除的是目录，关闭路径位于该目录下的所有 tab。

### 重命名行为

- 复用 `PromptDialog`。
- 默认值填入原文件名/目录名。
- 后端执行 `std::fs::rename(old, new)`。
- 重命名成功后刷新父目录。
- 如果重命名的是已打开 tab：
  - 更新 tab 的 `path` 和 `filename`；
  - 迁移 ref 桶 key：`oldPath -> newPath`；
  - 如果 activeTab 是 oldPath，则切到 newPath。
- 如果重命名的是目录：
  - 对所有 `path` 以旧目录为前缀的 tab 批量改成新前缀；
  - 同步迁移 refs。

### 在访达中打开

后端新增 `show_in_folder(path)`：

- macOS：`open -R <path>`；如果是目录，则 `open <dir>`。
- Windows：`explorer /select,<path>`；目录直接打开。
- Linux：优先 `xdg-open <dir>`。

命名上前端显示：

- macOS：在访达中显示 / 在访达中打开
- 其他平台：在文件管理器中显示 / 打开所在文件夹

## 具体改动文件

1. `apps/jstudio/src-tauri/src/lib.rs`
   - 新增 request/response 类型：
     - `PathReq { path }`
     - `RenameReq { path, new_name }`
     - `RenameResp { old_path, new_path }`
   - 新增 command：
     - `rename_path`
     - `delete_path`
     - `show_in_folder`
   - 注册到 `tauri::generate_handler!`。

2. `apps/jstudio/src/api.ts`
   - 新增：
     - `renamePath(path, newName)`
     - `deletePath(path)`
     - `showInFolder(path)`

3. `apps/jstudio/src/FileTree.tsx`
   - 扩展 props：
     - `onRenamePath`
     - `onDeletePath`
     - `onShowInFolder`
   - 扩展 `ContextMenuState`，记录：
     - `dir`
     - `entry?: DirEntry`
   - 右键文件/目录时显示对应菜单。
   - 新增 `renaming` prompt 状态。
   - 删除/重命名成功后刷新父目录/当前目录。

4. `apps/jstudio/src/Reader.tsx`
   - 引入新增 api。
   - 增加：
     - `renamePathAction`
     - `deletePathAction`
     - `showInFolderAction`
   - 实现 tab/ref 路径同步辅助函数。
   - 将动作传给 `FileTree`。

5. 可选：`apps/jstudio/src/Icon.tsx`
   - 如果已有合适 icon 则复用；否则新增 Rename/Delete/FolderOpen 类图标。

## 验证

执行：

```bash
cd apps/jstudio && npm run build
cd apps/jstudio/src-tauri && cargo fmt && cargo clippy -- -D warnings
```

必要时：

```bash
make install-jstudio
```

## 风险与处理

- 删除目录可能误删大量文件：必须二次确认，并在文案中区分文件/目录。
- 重命名 dirty tab：允许重命名，但 dirty 内容仍保留在 ref 中，保存时写入新路径。
- 后端路径安全：所有操作基于 canonicalize existing path；新文件名禁止路径分隔符，禁止覆盖已有目标。
