# clash

<!-- 项目使用入口与目录说明。修改时间：2026-07-28 18:15:12 +08:00 -->

## Quick Start
```sh
./shell/setup.sh
```
```sh
clash config add {name} -u '{url}'
```
```sh
clash on
```
注意
- 如果你的系统中"clash"已经在PATH或alias中，你只需修改setup.sh中注入的函数名，即可避免冲突。


## shell支持
注册
```sh
./setup.sh
```

使用
```sh
clash                    # 简要运行状态
clash status             # 详细状态
clash on                 # normal 模式启动，并注入当前 shell 的代理环境变量
clash tun                # TUN 模式启动，保留交互式 sudo 验证
clash off                # 关闭 mihomo，并清理当前 shell 的代理环境变量
clash env                # 按当前运行状态同步当前 shell 的代理环境变量
clash log                # 使用 less +F 动态查看最近一次运行日志
clash config add kitty -u 'https://example.com/sub'
clash config add local -f ~/Downloads/local.yaml
clash config set-url kitty -u 'https://example.com/new-sub'
clash config rm kitty
clash config ls
clash config cache-update
clash config cache-clean
```

`clash on` 会把 `config.yaml` 中的 `tun.enable` 写为 `false`，`clash tun` 会写为 `true`，因此可以交叉使用 normal 和 TUN 模式。

`setshell.sh` 会为 zsh 和 bash 注册提示符同步钩子。`clash on` 导入代理环境变量后，`config.yaml` 或 PID 状态发生变化会在下一个提示符自动同步；配置未变化时不会重复解析 YAML。

配置读取和修改由项目内置的 Python 工具完成，不依赖系统中的 `yq` 或额外 Python 包。运行环境只需要 Python 3.7+；round-trip YAML 依赖已经固定版本并放在 `shell/vendor/` 中。更新会先写入同目录临时文件，通过结构校验和可用的 mihomo `-t` 校验后再备份、原子替换，因此失败不会留下半份配置。

`clash config add` 写入成功后，如果检测到本目录的 mihomo 正在运行，会通过 external-controller 热加载配置，不会停止或重启服务；该步骤需要 `curl`。

`config add` 使用位置参数作为 Provider 名称，`-u` 添加 HTTP 订阅，`-f` 将本地文件原子复制到托管路径；未指定 `-p` 时统一使用 `./providers/{name}.yaml`。`config set-url` 只修改已有 HTTP Provider 的订阅链接。

`config ls` 会在 mihomo 运行时读取 Provider API，订阅信息只显示余额和到期日期；内核未运行或订阅未提供相关信息时显示 `remain - expire -`。

`config cache-update [名称 ...]` 通过运行中 Mihomo 的 Provider API 更新全部或指定的 HTTP Provider。`config cache-clean` 不直接删除文件，而是把 HTTP 缓存及孤立文件移动到 `logs/providers/providers_时间戳/`；仍被 File Provider 引用的文件会保留。

每次启动都会在 `logs/mihomo/` 创建独立的运行日志，`current.log` 始终指向最近一次运行。配置变更前的备份保存在 `logs/backup/`，不会由缓存清理命令删除。

## webui
访问`localhost:9090/ui`


## 目录结构
```sh
clash
├── logs                     # 运行日志、配置备份和安全归档
├── cache.db
├── config.yaml               # 主配置文件
├── Country.mmdb
├── GeoIP.dat
├── geosite.dat
├── mihomo-linux-amd64-v1     # 内核
├── providers                 # 订阅文件缓存
│   ├── ikuuu.yaml
│   └── kitty.yaml
├── ui                        # 管理面板
└── shell
    ├── clash                 # 单入口：启停、状态、日志、订阅配置
    ├── config.py             # round-trip YAML 配置管理与原子更新
    ├── provider_file.py      # File Provider 原子导入与失败回滚
    ├── provider_cache.py     # HTTP Provider 更新和缓存安全归档
    ├── vendor                # 内置的固定版本 Python YAML 依赖
    ├── tests                 # 隔离配置 fixture 与回归测试
    └── setshell.sh           # 向 zshrc/bashrc 注入 clash 函数
```
