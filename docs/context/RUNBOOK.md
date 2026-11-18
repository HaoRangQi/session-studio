# RUNBOOK —— 操作与验证

> 更新日期：2026-11-08
> 状态：源码实现中；合成样本自动化验证通过；原生 debug GUI 已启动并查看默认路径；真实数据与四源闭环尚未验收

## 前置条件

- macOS、Rust/Cargo、Node.js/npm。
- 前端开发与测试使用 Node.js/npm；本机当前只有 Command Line Tools。debug GUI 已在现有工具链上启动；完整发行构建和 macOS 系统集成仍待具备完整 Xcode 的环境验证。
- 来源路径按工具默认值显示；显示路径不读取内容，只有用户明确点击某来源“扫描”时才访问该单一目录。界面查看与开发启动不点击扫描；不得全盘扫描。

## 安装依赖

- 前端依赖已在实现 worktree 中通过 `npm install --ignore-scripts` 安装；`npm audit` 当前报告 0 vulnerabilities。
- Rust 依赖由 `src-tauri/Cargo.toml` 管理；Cargo test 与 desktop feature 检查已运行通过。

## 启动与运行

- 前端开发界面：在实现 worktree 执行 `npm run dev -- --port 1420 --strictPort`，Vite 仅监听 `127.0.0.1:1420`。2026-11-08 重启后 PID 86303 监听；`curl` 返回 Session Studio 页面。Tauri WebView 已实际加载此 dev URL。普通浏览器模式下 `isTauri()` 为 false，不调用来源状态或扫描命令。
- Tauri GUI：使用 `cargo run --manifest-path src-tauri/Cargo.toml --features desktop --bin session-studio` 启动 debug 窗口；本机仅有 Command Line Tools，但此命令已成功编译并显示窗口。修改前已在设置页核对四个默认路径。
- Tauri GUI 中 `get_sources` 只返回来源状态，不扫描目录；扫描仅由用户点击对应来源的“扫描”按钮触发。Claude Code/DeepSeek 格式未验证，活动检测和 CLI 分析未启用。查看 GUI 时禁止点击扫描。

## 验证步骤

| 对象 | 方法 | 状态 |
| --- | --- | --- |
| 产品/技术设计 | 用户逐节确认；上游格式路径通过官方仓库文档/源码核查 | 已确认/已核查，见设计说明 |
| 前端 | `npm test`、`npm run build` | 2026-11-08 通过：8 项测试；生产构建成功 |
| Rust/Tauri | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml --features desktop` | 2026-11-08 通过；Rust 单元/集成测试 34 项 |
| 依赖审计 | `npm audit` | 2026-11-08 通过：0 vulnerabilities |
| 四源真实闭环 | 每源用可授权样本验证发现、浏览、导出、垃圾篓清理、CLI 分析 | 未执行；Claude Code/DeepSeek 适配器、活动关联与 CLI 分析未完成 |

## 本轮未执行事项

- Tauri GUI 已启动；四个默认路径在设置页已核对，状态均为“尚未扫描”。修正文案后重启 Vite 导致 WebView 黑屏，后续进程未重新创建可见窗口，因此全局提示的新文案未完成截图复验。
- 未点击扫描；未读取真实 Agent 目录或会话正文。首次 GUI 启动继承了 shell 中设置的模型环境变量，但启动路径未调用模型；随后已在清除此类变量后重新启动。
- 未调用真实模型；未移动文件至垃圾篓。
- 未执行 Git 提交/推送、部署或系统级 Xcode 安装。

## 编写约定

- 仅写入有依据的操作步骤，注明前置条件、执行状态与验证方法。
- 未实际执行的命令标注“未执行”；文档中的步骤不构成执行授权。
