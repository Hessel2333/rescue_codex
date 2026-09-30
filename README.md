# Rescue Codex

Rescue Codex 是一个本地优先的 Codex 使用数据分析桌面应用。它基于 Tauri、React、Vite 和 SQLite 构建，用来导入本机 Codex 会话数据，并在仪表盘中查看会话、项目、工具调用、性能和工作流趋势。

## 功能

- 扫描默认 `~/.codex` 数据目录或手动导入 `json` / `jsonl` 文件。
- 汇总会话数量、消息数量、工具调用、耗时、Token 与错误信号。
- 按项目、时间范围和关键字筛选分析结果。
- 展示项目活跃度、并行窗口、性能趋势、相关性和搜索结果。
- 在会话详情中显示图片输入、工具返回图片与 Codex 生图结果，并支持大图预览。
- 兼容新版 Codex 的消息、搜索完成项、线程设置与 Token 使用记录，包括首响应耗时与缓存写入 Token。
- 自动同步时跳过未变化的会话文件，并清理不再引用的托管媒体缓存。
- 使用本地 SQLite 存储数据，不依赖云端服务。

## 技术栈

- Tauri 2
- Rust
- React 19
- TypeScript
- Vite
- Tailwind CSS
- SQLite / rusqlite

## 开发环境

需要安装：

- Node.js
- pnpm
- Rust toolchain
- Tauri 对应平台依赖

macOS 还需要 Xcode Command Line Tools：

```sh
xcode-select --install
```

安装依赖：

```sh
pnpm install
```

启动前端开发服务：

```sh
pnpm dev
```

启动 Tauri 桌面应用：

```sh
pnpm tauri dev
```

构建前端：

```sh
pnpm build
```

构建桌面应用：

```sh
pnpm tauri build
```

## 跨平台说明

项目已包含 Windows 和 macOS 的 Tauri hook 配置：

- Windows: `src-tauri/tauri.windows.conf.json`
- macOS: `src-tauri/tauri.macos.conf.json`

前端构建入口统一放在 `scripts/` 下，避免直接依赖 Windows `.cmd` shim。路径分析逻辑也同时兼容 Windows `\` 和 Unix `/` 分隔符。

## 数据位置

应用数据库会写入 Tauri 的应用数据目录。Codex 默认导入路径为：

```text
~/.codex
```

内嵌图片会提取到应用数据目录中的本地媒体缓存，数据库只保存引用。事件上下文采用字段白名单存储，不保留 `world_state`、`turn_context` 中与分析无关的完整指令内容。

## 更新后的数据同步

更新应用后，在导入页面重新扫描本机 Codex 数据，或等待已开启的自动同步完成。
解析规则升级时会自动重新处理旧版本导入的会话，补齐提示词、首响应时间和搜索统计，无需删除数据库。
历史会话需要保留原始 JSONL 文件才能补齐数据；未记录相应信息的指标仍可能为空。

## 验证

常用检查：

```sh
pnpm build
cd src-tauri
cargo test
```
