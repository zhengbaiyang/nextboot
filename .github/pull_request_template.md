## 变更摘要

<!-- 简要说明本次 PR 解决的问题和核心改动 -->

## 涉及模块

- [ ] `crates/boot-core`（UEFI NVRAM 底层读写与解析引擎）
- [ ] `crates/boot-cli`（命令行工具与 Linux `pkexec` 提权执行器）
- [ ] `src-tauri`（Tauri v2 桌面宿主）
- [ ] `src`（前端界面）
- [ ] `.github/workflows`（CI/CD 构建配置）

## 验证清单

- [ ] `cargo test -p boot-core -p boot-cli` 全部通过
- [ ] `npm run build` 前端类型检查与构建通过
- [ ] 已使用 `--mock` 模式完成端到端交互冒烟测试
- [ ] 如涉及 UEFI 写入逻辑，已在 Windows / Ubuntu 真机或模拟环境核验安全性
