# clash

## Quick Start
```sh
./shell/setup.sh
```
```sh
clash config add -n {name} -u '{url}' -p ./providers/{name}.yaml
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
clash log                # 使用 less +F 动态查看 mihomo.log
clash config add -n kitty -u 'https://example.com/sub' -p ./providers/kitty.yaml
clash config rm -n kitty
clash config ls
```

`clash on` 会把 `config.yaml` 中的 `tun.enable` 写为 `false`，`clash tun` 会写为 `true`，因此可以交叉使用 normal 和 TUN 模式。

配置修改依赖 `yq`：

```sh
sudo apt install yq
```

## webui
访问`localhost:9090/ui`


## 目录结构
```sh
clash
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
    └── setup.sh              # 向 zshrc/bashrc 注入 clash 函数
```
