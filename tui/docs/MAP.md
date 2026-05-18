# Docs Map

本文档维护 `/tui` 源码、配置、资料和文档之间的映射。修改源码前，先通过本文件找到需要阅读或同步更新的文档。

## 使用规则

- `Source Path` 指源码、配置、资料或工程入口路径。
- `Documentation` 指对应文档路径；如果暂未创建，使用建议路径。
- `Update When` 说明什么变化需要同步更新文档。
- 新增核心目录、公共模块、外部 API、运行流程或生成产物时，应同步更新本文件。

## 当前映射

| Source Path | Documentation | Update When |
| --- | --- | --- |
| `README.md` | `docs/README.md`, `docs/architecture/overview.md`, `docs/workflows/development.md` | 修改项目定位、启动方式、目录说明或验证命令时 |
| `Cargo.toml`, `Cargo.lock` | `docs/workflows/development.md`, `docs/architecture/overview.md` | 修改 Rust edition、依赖、feature、构建或运行要求时 |
| `src/main.rs` | `docs/architecture/overview.md`, `docs/modules/core.md`, `docs/workflows/development.md` | 修改 CLI 参数、启动入口、tokio runtime 或 TUI 入口时 |
| `src/runtime/` | `docs/architecture/overview.md`, `docs/modules/core.md` | 修改启动编排、工作目录发现、配置发现、mihomo 二进制发现或 `BootContext` 时 |
| `src/config/` | `docs/architecture/overview.md`, `docs/modules/core.md` | 修改 `runtime.yaml` 生成、`external-controller` 默认值、Provider 顺序、TUN 写入或用户配置保护规则时 |
| `src/mihomo/client.rs` | `docs/architecture/overview.md`, `docs/modules/core.md`, `docs/modules/mihomo-api.md` | 修改 external-controller API、认证、请求超时、`no_proxy()`、日志 WebSocket、测速或 Provider 刷新时 |
| `src/mihomo/process.rs` | `docs/architecture/overview.md`, `docs/modules/core.md` | 修改 mihomo 启停、PID 文件、sudo、子进程日志或 proxy 环境变量隔离时 |
| `src/mihomo/models.rs` | `docs/modules/core.md`, `docs/modules/mihomo-api.md` | 修改 mihomo 响应模型、serde 字段、兼容策略或新增 API 数据结构时 |
| `src/app/` | `docs/modules/core.md`, `docs/architecture/overview.md` | 修改应用状态、事件循环、快捷键、节点切换、Provider 列表、测速、日志窗口或 TUN/Proxy 动作时 |
| `src/events/` | `docs/modules/core.md` | 修改键鼠事件、Tick、日志桥接、后台任务结果或事件枚举时 |
| `src/ui/` | `docs/modules/core.md`, `docs/modules/ui-interaction.md` | 修改 ratatui 布局、页面结构、主题、高亮、日志浮层或交互文案时 |
| `docs/archive/` | `docs/modules/mihomo-api.md`, `docs/architecture/overview.md` | 更新 mihomo API 样例、历史接口资料或用于实现对照的参考数据时 |
| `docs/README.md`, `docs/MAP.md`, `docs/SPEC.md` | `docs/init.md` | 修改文档体系入口、映射规则、模板或文档质量要求时 |

## 已创建核心文档

```txt
docs/architecture/overview.md
docs/modules/core.md
docs/workflows/development.md
```

## 建议后续补齐

```txt
docs/modules/mihomo-api.md       # external-controller 端点、请求/响应、兼容策略
docs/modules/runtime-config.md   # runtime.yaml 生成、TUN、Provider 顺序和用户配置保护
docs/modules/ui-interaction.md   # 页面、快捷键、鼠标区域和渲染约束
docs/decisions/0001-runtime-first.md
```
