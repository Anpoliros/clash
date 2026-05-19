# 001. TUI 不控制 mihomo 启停

## 状态

Accepted

## 背景

早期 TUI 尝试在 General 页提供 Proxy/TUN 开关，由 TUI 负责启动、停止 mihomo，并在 TUN 场景中处理 sudo、runtime 配置修改和 reload。

实际使用中，这个方向不符合直觉：终端用户通常已经通过 shell、systemd、脚本或桌面入口管理 mihomo 生命周期。TUI 中的开关容易让人误以为它只是在切换代理状态，但实际会牵涉进程创建、权限、配置 reload、日志管道和 PID 文件维护。

## 决策

TUI 不再直接启动、停止或 reload mihomo。它只做三件事：

- 通过工作目录中的 `mihomo.pid` 展示运行状态。
- 通过 external-controller API 读取状态、切换节点、刷新 Provider、测速和读取日志。
- 生成运行时配置与保存 TUI 偏好，但不把生命周期控制绑定到 General 页开关。

## 原因

- 交互不直观：Proxy/TUN 开关看起来像普通设置，实际会触发进程和权限操作。
- 实现复杂：需要处理 sudo、raw mode 恢复、环境变量、子进程 stdout/stderr、PID 文件、reload 失败和配置回滚。
- Bug 面太大：终端环境、shell 启动脚本、systemd 服务和手动运行 mihomo 的路径差异会让 TUI 很难可靠判断“正确”的启停行为。
- 边界更清晰：TUI 作为 runtime client 更适合专注 external-controller 和节点体验。

## 后续恢复入口

如果未来确实要加回启停控制，建议不要直接把逻辑塞回 `app::activate` 或 General 表项中，而是新增独立模块：

- `src/mihomo/process_control.rs`：封装 start/stop/reload/sudo/raw-mode 边界。
- `src/app/` 中新增明确的命令入口，例如独立页面或确认弹窗，而不是伪装成普通 label/toggle。
- `docs/workflows/development.md` 增加人工验证矩阵，覆盖普通启动、systemd 管理、sudo TUN、reload 失败和 PID 不一致。

恢复前必须先定义清楚：TUI 是接管 mihomo 生命周期，还是只向外部服务管理器发命令。这两个模型不能混在同一个开关里。
