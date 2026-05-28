# Core Modules

本文档说明 `/tui/src` 下核心模块的职责、边界和修改入口。

## 模块视角

| Module | 职责 | 边界 |
| --- | --- | --- |
| `main.rs` | CLI 参数解析、启动上下文初始化、进入 TUI | 只支持 `-d/--dir`，不承载业务逻辑 |
| `runtime/` | 启动编排、配置发现、创建 `BootContext` | 不直接渲染 UI，不直接处理用户输入 |
| `config/` | 生成 `runtime.yaml`、补齐 controller、读取 Provider 顺序和 TUI 偏好 | 不写回用户原始 `config.yaml` |
| `mihomo/` | external-controller API、PID 状态、API 数据模型 | 不持有 UI 状态，不控制 mihomo 启停 |
| `events/` | 键鼠输入、Tick、日志桥接、后台任务结果统一为 `AppEvent` | 不决定业务动作 |
| `app/` | 应用状态、事件循环、业务动作、数据转换 | 不直接绘制 widget |
| `ui/` | ratatui 页面渲染、主题和日志浮层 | 不访问 mihomo API |

## 实现视角

### `runtime/`

`bootstrap(work_dir)` 是启动准备入口。它会：

- 在工作目录中优先查找 `config.yaml`、`config.yml`，否则选择排序后的第一个 YAML。
- 调用 `runtime_config::prepare` 生成 runtime 配置。
- 创建 `MihomoClient` 和 `MihomoProcess`。

### `config/`

`RuntimeConfig` 保存 runtime 配置路径、controller、secret 和 Provider 顺序。当前生成路径固定为：

```txt
~/.config/clash-tui/runtime.yaml
```

`prepare` 会复制用户 YAML，必要时补齐 `external-controller`。TUI 偏好配置保存在工作目录下的 `tui/config/ui.yaml`。

### `mihomo/`

`MihomoClient` 使用 `reqwest::ClientBuilder::no_proxy()`，避免继承终端代理环境变量。所有需要 secret 的请求通过 Bearer token 注入。

`MihomoProcess` 负责：

- 通过工作目录中的 `mihomo.pid` 判断运行状态。

TUI 不再负责启动、停止或 reload mihomo。原因和后续恢复入口见 `docs/decisions/001-no-tui-process-control.md`。

### `app/`

`App` 是业务中心，状态分为 `UiState`、`RuntimeState` 和 `MihomoState`。事件处理入口是 `handle`：

- `Key` 和 `Mouse` 转成页面切换、移动、展开、选择、刷新、测速和排序。
- `Tick` 同步进程状态，并定期刷新 mihomo API。
- `Log` 写入最多 1000 行日志缓冲。
- `DelayResult` 回填节点延迟。

节点切换由 `select_node` 完成。它优先选择真实节点组，再在根选择组上选择真实节点组，避免用户选中节点后仍被自动选择策略覆盖。需要回到 clash/mihomo 自动选择时，Proxies 页顶部的 `Auto Select` 行会把根选择组切回自动组。

Proxies 页水平键按当前行决定动作：Provider 行用 `h/l` 收起或展开；节点行用 `h/l` 在列间移动，当光标已在最左侧节点列时再次按 `h` 会收起所属 Provider 并把光标放回 Provider 行。

### `ui/`

`ui::draw` 组合三块区域：

- 顶部 Tab：`Proxies | General | Rules`。
- 中间页面：状态页、Provider/节点列表、Rules 占位页。
- 底部状态栏：当前状态和核心快捷键。

日志窗口是全屏浮层，由 `UiState.logs_open` 控制。

## 数据模型

`mihomo/models.rs` 只定义 MVP 用到的响应字段：

- `ConfigsResponse`：`port`、`mixed-port`、`tun.enable`。
- `ProxyItem`：`name`、`type`、`now`、`all`、`history`、`hidden`。
- `ProviderItem`：Provider 名称和真实节点列表。
- `DelayHistory`、`DelayResponse`、`VersionResponse`：测速和版本展示。

新增字段时优先使用 `#[serde(default)]` 保持兼容，避免不同 mihomo 版本缺字段导致 TUI 崩溃。

## 修改指南

- 改启动参数：从 `main.rs` 开始，同时检查 `docs/workflows/development.md`。
- 改 mihomo 工作目录发现：看 `runtime/bootstrap.rs`，确认错误信息和排序规则。
- 改 runtime 配置：看 `config/runtime_config.rs`，必须保持“不写回用户配置”的边界。
- 改 external-controller API：看 `mihomo/client.rs` 和 `mihomo/models.rs`，同步更新 API 文档或 `docs/MAP.md` 的建议文档。
- 恢复启停和 sudo：先读 `docs/decisions/001-no-tui-process-control.md`，再设计独立的 process-control 模块，不要直接塞回当前交互主线。
- 改快捷键或鼠标行为：看 `app::handle_key`、`app::handle_mouse` 和 `ui/mod.rs` 状态栏提示。
- 改 Provider/节点视图：看 `load_providers`、`build_from_real_providers`、`build_from_proxy_groups`、`rebuild_rows` 和 `draw_proxies`。

## 验证方式

```bash
cd /home/anpoliros/clash/tui
cargo fmt
cargo check
cargo run -- -d /home/anpoliros/clash
```

需要人工确认的场景：

- Proxies 页 Provider 能展开、刷新、测速、排序。
- 选择节点后 General 页当前节点和 Proxies 页高亮会同步；选择 `Auto Select` 后可以回到自动选择组。
- 日志浮层可以打开、滚动和关闭。

## 相关文档

- `docs/architecture/overview.md`
- `docs/workflows/development.md`
- `docs/MAP.md`
