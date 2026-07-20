# Architecture Overview

`clash-tui` 是面向 mihomo 的 Runtime First 终端客户端：优先通过 external-controller API 操作运行态，配置文件限定在用户传入的 mihomo 工作目录内。

## 系统边界

- 用户入口是 `src/main.rs`，只接受 `-d/--dir <mihomo-work-dir>`。
- mihomo 工作目录由用户传入，通常包含 mihomo 二进制、`config.yaml`、地理数据和 `providers/` 缓存。
- TUI 读取用户传入工作目录中的 `config.yaml`，不再生成 `~/.config` 下的 runtime 副本。
- Rules 管理会在备份后写回 `config.yaml` 的本地 `rule-providers` 和对应 `RULE-SET` 顺序。
- mihomo 运行态通过 `external-controller` 控制，默认补齐为 `127.0.0.1:9090`。
- HTTP client 会隔离常见 proxy 环境变量，避免访问本机 controller 被代理转发。
- TUI 不控制 mihomo 启停；原因见 `docs/decisions/001-no-tui-process-control.md`。

## 启动数据流

1. `main.rs` 解析 `-d/--dir` 并 canonicalize 工作目录。
2. `runtime::bootstrap::bootstrap` 查找 YAML 配置并加载 TUI 偏好。
3. `config::runtime_config::prepare` 读取用户配置中的 controller、secret 和 Provider 顺序。
4. `BootContext` 组合 `RuntimeConfig`、`MihomoClient` 和 `MihomoProcess`。
5. `app::run` 创建事件通道、日志通道、输入线程和日志 WebSocket 任务。
6. ratatui 进入 alternate screen，事件循环持续渲染并处理 `AppEvent`。

## 运行态数据流

`App` 明确拆分三类状态：

| State | 作用 |
| --- | --- |
| `UiState` | 当前页面、光标、滚动位置、日志窗口和状态栏文本 |
| `RuntimeState` | Proxy/TUN 状态、代理端口、管理端口、PID、当前节点 |
| `MihomoState` | mihomo 版本、根选择组、真实节点组、Provider 视图 |

周期性 `Tick` 会刷新进程状态；当 mihomo 正在运行且 tick 满足间隔时，重新读取用户配置中的 Provider 顺序，并同步 `/version`、`/configs`、`/proxies` 和 `/providers/proxies`。配置顺序只影响展示顺序，不会过滤 API 返回的新 Provider。

## 外部接口

当前 TUI 使用的 mihomo API 包括：

| API | 用途 |
| --- | --- |
| `GET /version` | 检查 controller 可用性并显示版本 |
| `GET /configs` | 同步代理端口和 TUN 状态 |
| `PUT /configs?force=true` | Rules 保存后热加载工作目录中的 `config.yaml` |
| `GET /proxies` | 获取 Selector、当前节点和 fallback Provider 数据 |
| `PUT /proxies/{group}` | 切换 Selector 选择 |
| `GET /proxies/{node}/delay` | 单节点测速 |
| `GET /providers/proxies` | 获取真实 Provider 和节点列表 |
| `PUT /providers/proxies/{provider}` | 刷新 Provider |
| `GET /providers/proxies/{provider}/healthcheck` | Provider 健康检查 |
| `WS /logs` | 流式日志 |

## 关键设计约束

- Runtime First：优先操作 mihomo API，不把 TUI 做成 YAML 编辑器。
- 配置保护：常规运行态只写 TUI 偏好配置；Rules 管理写回 `config.yaml` 前必须备份。
- 最小模型：`mihomo/models.rs` 只反序列化当前 MVP 需要的字段，未知字段交给 serde 忽略。
- Provider 顺序：优先从原始 YAML 文本读取 `proxy-providers` 顺序，避免 serde mapping 顺序不稳定影响 UI。
- 节点切换：先把真实节点组切到具体节点，再把根选择组切到真实节点组，使用户选择后流量固定到该节点；需要恢复自动选择时再把根选择组切回自动组。
- 启停边界：TUI 不处理 start/stop/sudo；Rules 只允许通过 external-controller 热加载配置，不停止或重启 mihomo。

## 相关文档

- `docs/modules/core.md`
- `docs/workflows/development.md`
- `docs/MAP.md`
