# clash-tui

`clash-tui` 是面向 mihomo 内核的现代化终端代理客户端。它不是 YAML 编辑器，而是 Runtime First 的代理控制台：优先通过 mihomo external-controller API 操作运行态，只在必要时生成和维护独立的 runtime 配置。

## 目标

- 终端内完成代理启停、TUN 开关、节点选择、订阅刷新、延迟测试和日志查看。
- 不修改用户原始 `config.yaml`。
- 不暴露过多 mihomo 内部概念，让用户直接面对“当前节点、延迟、端口、TUN、日志”。
- 不继承终端中的 `http_proxy` / `https_proxy` 等环境代理，避免访问 `127.0.0.1:9090` 被转发到本机代理端口导致 502。

## 运行

```bash
cd /home/anpoliros/clash/tui
cargo run -- -d /home/anpoliros/clash
```

`-d/--dir` 指向 mihomo 工作目录。该目录应包含：

- mihomo 二进制，例如 `mihomo-linux-amd64-v1`
- 用户配置，例如 `config.yaml`
- `Country.mmdb`、`GeoIP.dat`、`geosite.dat`
- `providers/` 等订阅缓存目录

## 目录结构

```text
src/
  main.rs              #----入口与参数解析----
  app/                 #----应用状态、事件循环、业务动作----
  ui/                  #----ratatui 渲染与主题----
  events/              #----键盘、鼠标、日志、后台任务事件----
  mihomo/              #----external-controller API 与进程控制----
  runtime/             #----启动编排、工作目录发现----
  config/              #----runtime.yaml 生成与 TUN 配置修改----
```

## 开发文档

`docs/` 是后续维护 `/tui` 的文档入口：

- `docs/README.md`：阅读顺序和文档目录说明。
- `docs/MAP.md`：源码路径到文档路径的映射。
- `docs/SPEC.md`：文档模板、更新规则和质量要求。
- `docs/architecture/overview.md`：整体架构、数据流和外部边界。
- `docs/modules/core.md`：核心模块职责和修改指南。
- `docs/workflows/development.md`：本地开发、验证和排障流程。

## 模块说明

### `main.rs`

解析唯一启动参数 `-d/--dir`，完成启动上下文初始化，然后进入 TUI 主循环。

### `runtime/`

负责启动前准备：

- 查找 mihomo YAML 配置。
- 查找 mihomo 可执行文件。
- 调用 `config::runtime_config` 生成 runtime 配置。
- 创建 `MihomoClient` 和 `MihomoProcess`。

### `config/`

负责配置文件层面的最小操作：

- 读取用户原始 YAML。
- 生成 `~/.config/clash-tui/runtime.yaml`。
- 补齐 `external-controller` 默认值。
- 从 `proxy-providers` 中读取 Provider 顺序。
- TUN 开关时只修改 runtime 配置，不写回用户原始配置。

### `mihomo/`

负责和 mihomo 交互：

- `client.rs` 封装 `/configs`、`/proxies`、`/providers/proxies`、`/logs`、`/version`。
- `process.rs` 负责启动和停止 mihomo 后端。
- `models.rs` 定义 MVP 需要的 API 数据结构。

`MihomoClient` 使用 `reqwest::ClientBuilder::no_proxy()`，不会继承环境变量中的代理配置。`MihomoProcess` 启动子进程时也会移除常见 proxy 环境变量。

### `events/`

把输入和后台结果统一转换为 `AppEvent`：

- 键盘事件
- 鼠标事件
- Tick 刷新事件
- 日志行
- 异步测速结果

### `app/`

应用核心，明确拆分三类状态：

- `UiState`：当前页面、光标、滚动位置、日志窗口状态。
- `RuntimeState`：Proxy/TUN 开关、端口、PID、当前节点。
- `MihomoState`：mihomo API 返回的 Provider、节点、活动分组。

这里也负责业务动作：

- 启停 mihomo。
- TUN 开关和 sudo 验证。
- Provider 展开/收起。
- 切换节点。
- 刷新订阅。
- 异步测速。
- 日志滚动。

### `ui/`

只负责渲染，不直接访问 mihomo：

- 顶部 `Proxies | General | Rules` Tab。
- General 状态页。
- Proxies Provider + 两列节点网格。
- Rules 占位页。
- 全屏日志弹窗。

## 工作流

### 启动流程

1. 用户执行 `clash-tui -d <mihomo-work-dir>`。
2. 程序查找用户配置和 mihomo 二进制。
3. 基于用户配置生成 `~/.config/clash-tui/runtime.yaml`。
4. 初始化 mihomo API client。
5. 检查 mihomo 是否已运行。
6. 进入 TUI。
7. 周期性同步 `/version`、`/configs`、`/proxies` 和 `/providers/proxies`。

### Proxy 开关

- 关闭：向已有 PID 发送 `kill`，并移除 `mihomo.pid`。
- 开启：使用 runtime 配置启动 mihomo。
- 如果 TUN 处于开启状态，启动前会临时退出 TUI，执行 `sudo -v` 让用户输入密码，然后通过 `sudo -n` 启动 mihomo。

### TUN 开关

1. 修改 `runtime.yaml` 中的 `tun.enable`。
2. 调用 `/configs` reload。
3. 如需开启 TUN，先执行 `sudo -v` 做交互式提权。

### 节点切换

UI 允许用户直接选择真实节点，但 mihomo 内部仍通过 Selector 完成。

当前实现会优先寻找：

- 根选择组：`节点选择`、`Proxy`、`GLOBAL`
- 真实节点组：`全部节点`、`GLOBAL` 或其他包含真实节点的 Selector

切换节点时执行两步：

1. `全部节点 -> 具体节点`
2. `节点选择 -> 全部节点`

这样用户选择某个节点后，所有流量会固定走该节点，不再继续走自动选择。

Proxies 页顶部提供 `Auto Select` 行，用于把根选择组切回 clash/mihomo 配置中的自动选择组。

### Provider 列表

Provider 顺序来自用户配置中的 `proxy-providers`。每个 Provider 可以展开/收起：

```text
[▼] kitty [Refresh] [Ping] [Sort]
        [[kitty]HK 01] 56ms       [[kitty]JP 01] 67ms
```

支持：

- `j/k` 或上下方向键移动。
- 光标在 Provider 行时，`h/l` 或左右方向键收起/展开 Provider。
- 光标在节点行时，`h/l` 或左右方向键切换同一行两列节点。
- `Enter` 切换节点或展开/收起 Provider。
- 鼠标点击节点和 Provider。
- 鼠标滚轮滚动长列表。

### 日志窗口

General 页选中 `Log` 后按 `Enter` 打开日志窗口。

- `Esc` / `q` 关闭日志窗口。
- 鼠标滚轮向上查看历史日志。
- 鼠标滚轮向下回到最新日志。

日志来源：

- 优先使用 mihomo `/logs` WebSocket。
- 由 TUI 启动的 mihomo 还会捕捉 stdout/stderr。

## 环境变量隔离

很多 shell 脚本会注入：

```bash
http_proxy=http://127.0.0.1:7890
https_proxy=http://127.0.0.1:7890
all_proxy=socks5://127.0.0.1:7890
```

如果 TUI 继承这些变量，访问 `127.0.0.1:9090` 可能会被转发到代理端口，最终出现 502。

因此：

- API client 使用 `no_proxy()`，完全禁用环境代理。
- 启动 mihomo 子进程时移除大小写 proxy 环境变量。

## 验证

```bash
cargo fmt
cargo check
cargo run -- -d /home/anpoliros/clash
```
