# MAP —— 目录、版本、配置与部署位置

> 更新日期：2026-11-08
> 核验范围：实现源码、合成样本测试与本机工具链；未读取真实会话正文

## 目录结构

| 路径 | 用途 | 状态 |
| --- | --- | --- |
| `./` | Session Studio 项目根目录 | Git 已初始化；源码位于隔离实现 worktree |
| `.worktrees/session-studio-implementation/` | 隔离的未提交实现分支 | 应用源码、测试和构建配置；本机 `.git/info/exclude` 忽略 |
| `src/` | React/TypeScript 桌面 UI 与前端命令 DTO | 来源设置、会话/项目/候选视图及基本清理确认流程 |
| `src-tauri/src/` | Rust/Tauri 命令、域模型、SQLite、provider 与导出/清理逻辑 | Pi/Codex 已有合成样本验证；Claude/DeepSeek 待验证 |
| `src-tauri/tests/` | Rust 集成测试 | 合成 fixture，不访问用户真实会话 |
| `docs/superpowers/specs/` | 已确认的产品/技术设计 | `2026-11-08-session-studio-design.md` |
| `docs/superpowers/plans/` | 分阶段实现和验证清单 | `2026-11-08-session-studio.md` |
| `docs/context/` | 项目状态、映射、操作、决策和风险 | 沿用并更新 |
| `.ccgui/prompts/` | 本地工具目录，具体用途待核实 | 当前空目录，未改动 |

## 版本与依赖

| 对象 | 版本/状态 | 来源 |
| --- | --- | --- |
| Tauri | 2；desktop feature `cargo check` 已通过；debug GUI 已启动并检查设置页 | `src-tauri/Cargo.toml`；2026-11-08 验证 |
| Rust/Cargo | 本机 rustc 1.95.0 / cargo 1.95.0；测试与格式检查通过 | 2026-11-08 命令输出 |
| Node.js/npm | 本机 Node.js 26.11.1 / npm 11.20.0；测试/build/audit 通过 | 2026-11-08 命令输出 |
| 前端 | React + TypeScript + Vite；Vite/Tauri dev URL 为 `127.0.0.1:1420` | `package.json`、`vite.config.ts`、`src-tauri/tauri.conf.json` |
| 元数据/任务存储 | SQLite（`rusqlite`）；仅会话元数据入索引 | `src-tauri/src/storage/mod.rs` 与测试 |
| 项目版本 | `0.1.0` | `package.json`、`src-tauri/tauri.conf.json` |

## 本机 Agent CLI

| 工具 | 可执行文件状态 | 会话路径/格式证据 |
| --- | --- | --- |
| Claude Code | `claude` 可用 | `~/.claude/projects` 存在；格式契约待样本验证 |
| Codex | `codex` 可用 | 官方源码：`CODEX_HOME`（默认 `~/.codex`）下 `sessions/`、`archived_sessions/`；rollout JSONL 可有 `.jsonl.zst` 压缩 |
| DeepSeek Harness | 桌面应用进程可见；`dsh` 不在 PATH | CLI 配置默认 `DSH_HOME`（缺省 `~/.dsh`），会话目录为 `sessions/`；格式版本化 JSONL，默认 Zstandard；本机真实样本待验证 |
| Pi | `pi` 可用 | `~/.pi/agent/sessions` 存在；官方 JSONL v1-v3，按 cwd 分目录 |

## 配置与部署

- 本地 SQLite：Tauri `app_data_dir()` 下的 `session-studio.sqlite3`；数据目录在 GUI 首次运行时创建。
- Agent 会话数据：保留在各来源原目录；不复制到应用数据库。
- 分发：自用/小范围内测；不纳入 App Store、公证与签名配置。
- 桌面构建：当前只有 Xcode Command Line Tools；debug GUI 已成功编译和运行，完整发行构建与 macOS 系统集成仍待具备完整 Xcode 的环境验证。

## 关系说明

- 每个来源适配器负责发现、头部元数据、按需正文解析、活动关联与容器路径；共享核心提供统一索引和视图。
- DeepSeek 默认 root 按 `DSH_HOME` 配置解析为 `sessions/`；自定义目录可选，真实本机 root 与格式仍待样本验证。
- 上游格式和产品决策见 [设计说明](../superpowers/specs/2026-11-08-session-studio-design.md)。
