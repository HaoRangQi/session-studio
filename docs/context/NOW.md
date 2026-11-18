# NOW —— 当前状态

> 更新日期：2026-11-08
> 核验范围：产品设计、项目文档、隔离实现分支与当前构建环境

## 当前目标

- 实施已确认的 Session Studio 单机 macOS 应用设计；完整设计见 [设计说明](../superpowers/specs/2026-11-08-session-studio-design.md)。
- 首发目标包括 Claude Code、Codex、DeepSeek Harness、Pi 四种会话来源；四源真实闭环验收尚未完成。

## 进展

- 已确认产品范围、固定侧栏布局、会话容器级清理安全规则、Markdown/JSON 导出、Agent CLI 分析边界和本地索引策略。
- 已建立实施计划：[Session Studio 实施计划](../superpowers/plans/2026-11-08-session-studio.md)。
- 隔离 worktree 中已建立 React/Tauri 桌面壳、Rust 共享域模型、SQLite 元数据索引、默认/自定义来源目录扫描命令、候选查询、导出和清理预览流程；未提交。
- 四种来源均解析并显示工具默认目录；用户可选自定义或恢复默认，点击扫描前不读来源目录。Codex 扫描其 CODEX home 下 `sessions/` 与 `archived_sessions/`。
- Pi 与 Codex（含 `.jsonl.zst`）适配器通过合成样本验证；Claude Code 和 DeepSeek Harness 格式仍待验证。候选页使用独立后端查询。
- 活动监控和 CLI 分析尚未启用；当前活动快照不可靠，因此清理执行会保守阻止容器。
- 当前会话验证：前端 8 项测试通过，Rust 单元/集成测试 34 项通过，`npm run build`、`cargo fmt --check`、Tauri desktop feature `cargo check` 通过。仅使用合成 fixture；未扫描真实 Agent 目录、读取会话正文或调用模型。首次 GUI 启动继承了 shell 中的模型环境变量，但启动路径不发起模型请求；后续已清除这些变量再启动。

## 阻塞

- 本机仅有 Command Line Tools，但现有 debug target 已成功启动原生 GUI 并查看设置页；完整发行构建、系统文件访问错误处理与垃圾篓集成仍未验证。最近前端热重载后 WebView 变黑，修正后的全局提示尚未完成窗口截图复验。
- Claude Code 与 DeepSeek Harness 会话格式仍待验证；`dsh` CLI 在当前环境未安装。DeepSeek 默认目录按 `DSH_HOME`（默认 `~/.dsh`）解析为 `sessions/`；自定义目录不是扫描前置条件。
- 活动进程到会话容器的可靠关联、Agent CLI 分析执行闭环、真实四源样本验收均未完成，不能宣称 MVP 已交付。

## 下一步

1. 复核热重载后的全局状态提示：当前 Vite 已重启，但原生 WebView 未重新显示窗口；修复后的文案已有前端测试与构建验证。目录列表已在原生设置页实际核验，四源均未扫描。
2. 在用户明确提供的受控样本中验证 Claude Code 与 DeepSeek Harness 格式；单独确认实际访问边界与样本范围。
3. 实现并验证活动进程关联与 CLI 分析的安全闭环；模型调用与凭证访问保持关闭，直至另行确认。
4. 在具备完整 Xcode 的 macOS 环境完成原生 GUI、系统权限错误处理与垃圾篓集成验收，再进行四源真实样本闭环。

## 证据入口

- [README.md](../../README.md)：项目用途与使用状态
- [PROJECT_INDEX.md](../../PROJECT_INDEX.md)：任务路由
- [设计说明](../superpowers/specs/2026-11-08-session-studio-design.md)：确认的产品与技术要求
- [实施计划](../superpowers/plans/2026-11-08-session-studio.md)：分阶段实现与测试
- [MAP.md](MAP.md)：技术栈和已验证环境映射
- [RISKS.md](RISKS.md)：证据化阻塞与保护措施
