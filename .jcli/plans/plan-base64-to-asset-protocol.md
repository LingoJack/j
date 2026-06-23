# Base64 → Asset Protocol 改造

## 目标
用 `convertFileSrc()` 替换所有 Base64 data URL，让 WebView 直接通过 asset protocol 加载本地文件。

## 改动清单

### 1. tauri.conf.json — 启用 asset protocol
### 2. Rust: 新增 `get_doc_asset_path` 命令，返回资产绝对路径
### 3. 前端 storage.ts: 新增 `getDocAssetPath` 方法
### 4. upload.ts: `bytesToDataUrl` / `fileToDataUrl` → 改为返回 asset URL
### 5. editorUpload.ts: 适配新返回类型
### 6. ImageView.tsx / FileView.tsx: src 改用 asset URL
### 7. docxPreview.ts: 接受 asset URL 而非 data URL
### 8. PdfPreview.tsx: 适配 asset URL
### 9. previewWindow.ts: payload 改为传路径而非 data URL
### 10. PreviewWindowApp.tsx: 适配新 payload
### 11. fileUtils.ts: `ensureUtf8Charset` 适配 asset URL（no-op）
### 12. 向后兼容：旧文档中的 data URL 仍需正常显示
