# Development Workflow

<!-- 修改时间：2026-07-28 18:15:12 +08:00 -->

本文档记录 `clash-tui` 的本地开发、运行、验证和常见排障方式。

## 触发场景

- 修改 `/tui/src` 下任何核心模块。
- 更新 Rust 依赖、Cargo 配置或运行命令。
- 调整 mihomo 工作目录、配置读取、TUI 偏好、日志、Rules 或节点切换流程。
- 需要复现用户在终端里的代理体验问题。

## 输入与产物

输入：

- `-d/--dir <mihomo-work-dir>`：mihomo 工作目录。
- 工作目录中的 YAML 配置：优先 `config.yaml`、`config.yml`。

运行时产物：

- `<mihomo-work-dir>/tui/config/ui.yaml`：TUI 偏好配置。
- `<mihomo-work-dir>/rules/*.yaml`：Rules 页管理的本地 rule-provider 文件。
- `<mihomo-work-dir>/mihomo.pid`：由外部启动流程维护，TUI 只读取它来识别运行状态。

## 基础命令

```bash
cd /home/anpoliros/clash/tui
cargo fmt
cargo check
cargo run -- -d /home/anpoliros/clash
```

`-d/--dir` 必须指向 mihomo 工作目录，而不是 `/tui` 目录。

## 开发步骤

1. 阅读 `docs/MAP.md`，确认目标源码路径对应的文档。
2. 阅读 `docs/architecture/overview.md` 和相关模块文档。
3. 修改代码后运行 `cargo fmt`。
4. 运行 `cargo check` 确认类型和依赖无误。
5. 使用真实 mihomo 工作目录运行 `cargo run -- -d <dir>`。
6. 根据改动范围人工检查 General、Proxies、日志窗口、节点布局配置或 Provider 操作。

## 手工检查清单

| 改动范围 | 检查点 |
| --- | --- |
| CLI / bootstrap | 缺参、错误目录、缺配置时错误信息清晰 |
| config | controller、secret 和 Provider 顺序读取正常；没有生成 `~/.config` runtime 副本 |
| TUI config | `tui/config/ui.yaml` 被生成；General 中的布局参数可调且能持久化 |
| Rules | 分组启停和顺序会写入 `rules:` 顶部；规则文件在 `rules/*.yaml`；保存后热加载 |
| mihomo client | controller 请求不走环境代理；secret 场景仍能认证 |
| process | 读取 `mihomo.pid` 后能正确显示运行状态；不触发启停 |
| Proxies | Provider 顺序稳定；展开、选择、刷新、测速、排序和订阅余额显示正常 |
| UI | Tab、状态栏、日志浮层、鼠标滚轮和键盘移动不互相干扰 |

## 常见问题

### 访问 controller 返回 502 或连接到错误端口

优先确认改动是否破坏了代理环境隔离：

- `MihomoClient` 必须保留 `no_proxy()`。
- 如果要恢复启停控制，先读 `docs/decisions/001-no-tui-process-control.md`，不要直接复用当前 UI 主流程。

### Provider 列表为空

先检查 `/providers/proxies` 是否返回真实 Provider。若没有返回，当前实现会 fallback 到 `/proxies` 中非隐藏且包含节点的组。

### 当前节点没有按用户选择固定

检查根选择组和真实节点组选择逻辑：

- 根选择组优先：`节点选择`、`Proxy`、`GLOBAL`。
- 真实节点组优先：`全部节点`、`GLOBAL` 或其他含真实节点的 Selector。
- 切换时应先选真实节点，再让根选择组指向真实节点组。

## 验证方式

最小验证：

```bash
cd /home/anpoliros/clash/tui
cargo fmt -- --check
cargo check
```

交互验证：

```bash
cd /home/anpoliros/clash/tui
cargo run -- -d /home/anpoliros/clash
```

交互验证完成后，确认终端退出时 raw mode 恢复正常，工作目录中没有产生意外文件，用户原始配置没有变化。
