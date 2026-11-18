# RISKS —— 风险与验证缺口

> 更新日期：2026-11-08

## 风险清单

| 编号 | 问题/风险 | 证据 | 影响 | 保护措施 | 状态 |
| --- | --- | --- | --- | --- | --- |
| R-001 | 当前环境缺完整 Xcode/macOS SDK | 2026-11-08 `xcodebuild` 报告 active developer directory 为 Command Line Tools；debug GUI 已成功运行 | 完整发行包构建与 macOS 系统集成验收仍无法在当前环境确认 | 不改系统 developer directory；发行包和系统集成验证转到具备完整 Xcode 的 macOS 环境 | 阻塞验证 |
| R-002 | DeepSeek Harness CLI 入口与真实样本格式尚未验证 | `dsh` 不在 PATH；进程表可见 DeepSeek Harness 桌面应用及其 bundled runtime；CLI 配置层按 `DSH_HOME`（默认 `~/.dsh`）提供 `sessions/` root，persistence API 本身要求调用方传 root | 不能据桌面进程或默认路径推断本机真实会话位置或格式兼容性 | 按默认规则显示路径；格式未验证时明确标注 `needs_validation`，不伪装为空扫描或支持清理 | 待验证 |
| R-003 | Claude Code 本机格式仍需真实样本验证 | 已知本机 `~/.claude/projects` 存在，但公开 README 未提供稳定 session 格式契约 | 不能保证跨版本解析正确或安全清理粒度 | 默认目录可显示并按需扫描；先验证受控样本和容器边界，不猜格式 | 待验证 |
| R-004 | Claude Code/DeepSeek Harness 格式和 Codex/Pi 后续版本仍需真实兼容性验证 | Pi 与 Codex（含 `.jsonl.zst`）合成 fixture 测试通过；Claude/DeepSeek adapter 明确保持未验证 | 未识别变体可能导致误判空会话或读错元数据 | 有界解析；未验证格式不标记为可用，禁止据此清理 | 部分实现；真实样本待验证 |
| R-005 | 活动进程与会话容器可能无法可靠关联 | Tauri `get_activity` 当前固定返回 `reliable: false`；清理集成测试覆盖未知状态阻断 | 过度阻止清理或误移走正在写入的容器 | 当前阻止所有清理；实现可靠关联前不放宽 | 阻塞功能 |
| R-006 | CLI 分析执行器尚未完成安全参数验证 | Tauri 分析命令当前显式拒绝执行；尚无 fake CLI 安全语义闭环 | 错误参数可能触发写入、联网或读取未选内容 | 保持功能禁用；验证只读及外部处理风险后再开放 | 阻塞功能 |

## 验证缺口

- 当前自动化验证仅覆盖合成数据和 Rust/Tauri desktop crate 编译；不等于 macOS GUI 或系统集成验收。
- 四个来源都还没有完成真实样本的发现、浏览、导出、垃圾篓清理和 CLI 分析闭环。
- 产品设计和上游文档不等于本机集成已验证；未验证能力不得在 UI 中标为可用。

## 维护说明

- 只记录有证据的问题与风险；状态变化时更新本行，不保留矛盾快照。
