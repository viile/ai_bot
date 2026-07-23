# AI群聊（Tauri 桌面端）

本地桌面群聊应用：创建群、添加多个 Cursor 机器人（头像 / 昵称 / 身份设定）；用户发言后先做**上下文路由**选出最合适的角色再回复，避免全员齐刷相似内容。

通过 **Tauri** 在本机直接 `spawn cursor-agent`，无需 Express / WebSocket 代理。

消息路由与回复的完整链路见：[docs/message-flow.md](docs/message-flow.md)。

## 前置条件

1. Node.js 18+
2. Rust（[rustup](https://rustup.rs/)）
3. 本机可用 `cursor-agent`，且已 `cursor-agent login`（或设置 `CURSOR_API_KEY`）

```bash
cursor-agent status
```

## 快速开始

```bash
cp .env.example .env   # 可选
npm install
npm run dev            # 等价于 tauri dev
```

会启动 Vite 前端 + Tauri 窗口。

打包：

```bash
npm run tauri:build
```

## 使用说明

1. 左侧新建群聊
2. 「管理机器人」添加角色：昵称、头像、身份标签（年龄 / 职业 / 性格 / 国籍等）与身份设定、可选模型
3. 发消息 → 先静默路由选出合适机器人 → 入选者生成回复（仅用户消息触发，不级联）

## 数据

本地 SQLite：`data/ai_bot.sqlite`（开发环境在项目 `data/`；正式包在应用数据目录）。

表：`groups` / `bots` / `messages`。重启后群聊、聊天记录、身份设定都会保留。

若存在旧版 JSON（`groups.json` 等），首次启动会自动导入并挪到 `data/json_backup/`。

## 环境变量

可在启动前导出，或写入 shell profile：

| 变量 | 说明 |
|------|------|
| `CURSOR_AGENT_BIN` | cursor-agent 路径 |
| `CURSOR_MODEL` | 默认模型 |
| `CURSOR_TIMEOUT` | 超时毫秒（默认 120000） |
| `CURSOR_API_KEY` | API Key |

## 架构

- **Vue 3**：群列表、消息流、机器人管理
- **Tauri 2（Rust）**：SQLite 持久化、发言路由、`cursor-agent` 调用、`chat-event` 事件推送
- **链路说明**：[docs/message-flow.md](docs/message-flow.md)
