# clash-tui Docs

本目录是 `clash-tui` 的开发文档入口，目标是让维护者和 coding agent 在修改 `/tui` 前快速理解模块边界、运行流程和验证方式。

`clash-tui` 是面向 mihomo 的 Runtime First 终端客户端。文档应围绕运行态控制、mihomo API、进程管理、runtime 配置和 ratatui 交互来组织，而不是按每个源码文件机械复述。

## 文档目标

- 说明 `/tui` 的关键目录、模块职责和核心数据流。
- 记录 mihomo external-controller、工作目录配置、进程启停、TUN、日志和节点切换的维护边界。
- 帮助 coding agent 在改动前确认应该阅读的源码和文档。
- 为后续新增功能沉淀稳定模板，避免隐含知识只留在代码里。

## 目录组织

```txt
docs/
  README.md                    # 文档入口：如何阅读和维护文档
  MAP.md                       # 文档地图：源码路径到文档路径的映射
  SPEC.md                      # 文档规范：模板、更新规则和质量要求
  init.md                      # 从零启动 docs/ 体系的通用说明
  architecture/overview.md     # TUI 架构、数据流和外部边界
  decisions/                   # 重要设计决策和取舍记录
  modules/core.md              # app/runtime/config/mihomo/events/ui 核心模块说明
  workflows/development.md     # 开发、运行、验证和排障流程
  archive/                     # mihomo API 样例、历史资料和原始参考
```

后续可按需要扩展：

```txt
docs/modules/mihomo-api.md
docs/modules/runtime-config.md
docs/modules/ui-interaction.md
docs/workflows/release.md
docs/decisions/001-no-tui-process-control.md
```

## 阅读顺序

1. 修改前先读 `docs/MAP.md`，确认目标源码路径对应哪些文档。
2. 初次接触项目时读 `docs/architecture/overview.md`，建立整体视角。
3. 修改核心逻辑时读 `docs/modules/core.md`，确认模块职责和边界。
4. 需要运行、检查或排障时读 `docs/workflows/development.md`。
5. 新增或更新文档时遵循 `docs/SPEC.md`。

## 文档维护规则

- 修改核心模块、公共数据模型、mihomo API 契约、runtime 配置、启停流程或验证命令时，必须检查 `docs/MAP.md` 和相关文档。
- 文档只记录稳定事实、设计边界和修改风险，不追求覆盖每一行实现。
- `docs/archive/` 可保存原始 API 样例或历史资料，但不能替代当前实现文档；引用 archive 内容时要说明它只是参考。
