# Rescue Codex

Rescue Codex 是一个本地优先的 Codex 使用数据分析桌面应用。它基于 Tauri、React、Vite 和 SQLite 构建，用来导入本机 Codex 会话数据，并在仪表盘中查看会话、项目、工具调用、性能和工作流趋势。

## 功能

- 扫描默认 `~/.codex` 数据目录或手动导入 `json` / `jsonl` 文件。
- 汇总会话数量、消息数量、工具调用、耗时、Token 与错误信号。
- 按项目、时间范围和关键字筛选分析结果。
- 展示项目活跃度、并行窗口、性能趋势、相关性和搜索结果。
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

## 验证

常用检查：

```sh
pnpm build
cd src-tauri
cargo test
```
