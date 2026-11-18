# session-studio

## 项目用途

Session Studio 是一个单机 macOS 桌面应用，用于统一管理 Claude Code、Codex、DeepSeek Harness 和 Pi 的本机会话：发现与浏览会话、移入 macOS 垃圾篓、导出 Markdown/JSON、按需调用用户选择的 Agent CLI 分析，并只读监控受支持 CLI 的活动进程。

产品与技术设计已确认，详细范围、数据安全边界和真实样本验收标准见 [设计说明](docs/superpowers/specs/2026-11-08-session-studio-design.md)。应用源码、Rust 核心和基础界面已建立；目前只有部分合成样本路径通过自动化验证，不代表四种来源真实验收或 MVP 已交付。

## 基本使用方式

当前可在 Node.js/npm 环境启动前端开发界面：`npm install` 后运行 `npm run dev`，默认仅监听 `127.0.0.1:1420`。Tauri 桌面壳需要 macOS、Rust/Cargo、Tauri 2 开发依赖和完整 Xcode；本机当前只有 Command Line Tools，GUI 构建及运行尚未验证。不要将未经验证的来源连接到真实会话目录。

## 文档入口

| 文档 | 内容 |
| --- | --- |
| [AGENTS.md](AGENTS.md) | 长期规则与读写约定（接手项目必读） |
| [PROJECT_INDEX.md](PROJECT_INDEX.md) | 任务 → 文件或章节的路由 |
| [docs/context/NOW.md](docs/context/NOW.md) | 当前目标、进展、阻塞与下一步 |
| [docs/context/MAP.md](docs/context/MAP.md) | 目录、版本、配置与部署位置 |
| [docs/context/RUNBOOK.md](docs/context/RUNBOOK.md) | 操作、前置条件与验证步骤 |
| [docs/context/DECISIONS.md](docs/context/DECISIONS.md) | 已确认决策与理由 |
| [docs/context/RISKS.md](docs/context/RISKS.md) | 风险与验证缺口 |
| [docs/context/history/README.md](docs/context/history/README.md) | 历史记录索引 |

## 状态

- 当前仍处于开发验证阶段：Pi 与 Codex 有合成 JSONL 样本测试；Claude Code、DeepSeek Harness 格式和四源真实闭环尚未验证。
- 活动进程关联尚未实现，应用当前会保守阻止所有清理执行；CLI 分析也尚未启用。
