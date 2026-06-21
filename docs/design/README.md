# 句读设计资产

本目录保存从 `reference/` 整理出来的设计规格、原型 HTML、运行辅助脚本与截图。

- `句读 Design Spec.dc.html`：设计规格源文件。
- `句读 Prototype.dc.html`：可交互原型源文件。
- `support.js`：上述 HTML 依赖的运行辅助脚本。
- `screenshots/`：原型截图留档。

运行时代码只直接导入 `src/assets/design/tokens.css`。本目录作为后续前端页面实现和视觉验收的对照源，不进入 Tauri 运行时包。
