APIs - 虚空终端 Docs






[跳转至](#api)

[![logo](../assets/images.jpeg)](.. "虚空终端 Docs")




虚空终端 Docs

APIs




* [简体中文](./)
* [English](../en/api/)
* [Русский](../ru/api/)



正在初始化搜索引擎

[MetaCubeX/mihomo](https://github.com/MetaCubeX/mihomo "前往仓库")

* [简介](..)
* [安装](../startup/)
* [手册](../handbook/)
* [配置](../config/)
* [APIs](./)



[![logo](../assets/images.jpeg)](.. "虚空终端 Docs")
虚空终端 Docs

[MetaCubeX/mihomo](https://github.com/MetaCubeX/mihomo "前往仓库")

* [简介](..)
* [安装](../startup/)

  安装
  + [常见问题](../startup/faq/)
  + [客户端](../startup/client/)
  + [web面板](../startup/web/)
  + [创建运行服务](../startup/service/)
  + [三方工具/客户端](../startup/client/client/)
* [手册](../handbook/)

  手册
  + [语法](../handbook/syntax/)
  + [快捷配置](../example/conf/)
* [配置](../config/)

  配置
  + [全局配置](../config/general/)
  + [DNS](../config/dns/)

    DNS
    - [DNS类型](../config/dns/type/)
    - [hosts](../config/dns/hosts/)
    - [解析流程](../config/dns/diagram/)
  + [域名嗅探](../config/sniff/)
  + [入站](../config/inbound/)

    入站
    - [代理端口](../config/inbound/port/)
    - [Tun](../config/inbound/tun/)
    - [listeners](../config/inbound/listeners/)

      listeners
      * [http](../config/inbound/listeners/http/)
      * [socks](../config/inbound/listeners/socks/)
      * [mixed](../config/inbound/listeners/mixed/)
      * [redirect](../config/inbound/listeners/redirect/)
      * [tproxy](../config/inbound/listeners/tproxy/)
      * [tun](../config/inbound/listeners/tun/)
      * [ShadowSocks](../config/inbound/listeners/ss/)
      * [VMess](../config/inbound/listeners/vmess/)
      * [VLESS](../config/inbound/listeners/vless/)
      * [Trojan](../config/inbound/listeners/trojan/)
      * [AnyTLS](../config/inbound/listeners/anytls/)
      * [Mieru](../config/inbound/listeners/mieru/)
      * [Sudoku](../config/inbound/listeners/sudoku/)
      * [TUIC v4](../config/inbound/listeners/tuic-v4/)
      * [TUIC v5](../config/inbound/listeners/tuic-v5/)
      * [Hysteria2](../config/inbound/listeners/hysteria2/)
      * [TrustTunnel](../config/inbound/listeners/trusttunnel/)
      * [tunnel](../config/inbound/listeners/tunnel/)
  + [出站代理](../config/proxies/)

    出站代理
    - [TLS配置](../config/proxies/tls/)
    - [传输层配置](../config/proxies/transport/)
    - [dialer-proxy](../config/proxies/dialer-proxy/)
    - [内置代理策略](../config/proxies/built-in/)
    - [DIRECT](../config/proxies/direct/)
    - [DNS](../config/proxies/dns/)
    - [HTTP](../config/proxies/http/)
    - [SOCKS](../config/proxies/socks/)
    - [Shadowsocks](../config/proxies/ss/)
    - [ShadowsocksR](../config/proxies/ssr/)
    - [Snell](../config/proxies/snell/)
    - [VMess](../config/proxies/vmess/)
    - [VLESS](../config/proxies/vless/)
    - [Trojan](../config/proxies/trojan/)
    - [AnyTLS](../config/proxies/anytls/)
    - [Mieru](../config/proxies/mieru/)
    - [Sudoku](../config/proxies/sudoku/)
    - [Hysteria](../config/proxies/hysteria/)
    - [Hysteria2](../config/proxies/hysteria2/)
    - [TUIC](../config/proxies/tuic/)
    - [WireGuard](../config/proxies/wg/)
    - [SSH](../config/proxies/ssh/)
    - [MASQUE](../config/proxies/masque/)
    - [TrustTunnel](../config/proxies/trusttunnel/)
  + [代理集合](../config/proxy-providers/)

    代理集合
    - [代理集合内容](../config/proxy-providers/content/)
  + [代理组](../config/proxy-groups/)

    代理组
    - [内置代理组](../config/proxy-groups/built-in/)
    - [手动选择](../config/proxy-groups/select/)
    - [自动选择](../config/proxy-groups/url-test/)
    - [自动回退](../config/proxy-groups/fallback/)
    - [负载均衡](../config/proxy-groups/load-balance/)
    - [链式代理](../config/proxy-groups/relay/)
  + [路由规则](../config/rules/)
  + [规则集合](../config/rule-providers/)

    规则集合
    - [规则集合内容](../config/rule-providers/content/)
  + [子规则](../config/sub-rule/)
  + [流量隧道](../config/tunnels/)
  + [NTP](../config/ntp/)
  + [实验性配置](../config/experimental/)
* APIs

  [APIs](./)



  目录
  + [请求示例](#_1)
  + [日志](#_2)

    - [/logs](#logs)
  + [流量信息](#_3)

    - [/traffic](#traffic)
  + [内存信息](#_4)

    - [/memory](#memory)
  + [版本信息](#_5)

    - [/version](#version)
  + [缓存](#_6)

    - [/cache/fakeip/flush](#cachefakeipflush)
    - [/cache/dns/flush](#cachednsflush)
  + [运行配置](#_7)

    - [/configs](#configs)
    - [/configs/geo](#configsgeo)
    - [/restart](#restart)
  + [更新](#_8)

    - [/upgrade](#upgrade)
    - [/upgrade/ui](#upgradeui)
    - [/upgrade/geo](#upgradegeo)
  + [策略组](#_9)

    - [/group](#group)
    - [/group/group\_name](#groupgroup_name)
    - [/group/group\_name/delay](#groupgroup_namedelay)
  + [代理](#_10)

    - [/proxies](#proxies)
    - [/proxies/proxies\_name](#proxiesproxies_name)
    - [/proxies/proxies\_name/delay](#proxiesproxies_namedelay)
  + [代理集合](#_11)

    - [/providers/proxies](#providersproxies)
    - [/providers/proxies/providers\_name](#providersproxiesproviders_name)
    - [/providers/proxies/providers\_name/healthcheck](#providersproxiesproviders_namehealthcheck)
    - [/providers/proxies/providers\_name/proxies\_name/healthcheck](#providersproxiesproviders_nameproxies_namehealthcheck)
  + [规则](#_12)

    - [/rules](#rules)
    - [/rules/disable](#rulesdisable)
  + [规则集合](#_13)

    - [/providers/rules](#providersrules)
    - [/providers/rules/providers\_name](#providersrulesproviders_name)
  + [连接](#_14)

    - [/connections](#connections)
    - [/connections/:id](#connectionsid)
  + [域名查询](#_15)

    - [/dns/query](#dnsquery)
  + [DEBUG](#debug)

    - [/debug/gc](#debuggc)
    - [/debug/pprof](#debugpprof)

目录

* [请求示例](#_1)
* [日志](#_2)

  + [/logs](#logs)
* [流量信息](#_3)

  + [/traffic](#traffic)
* [内存信息](#_4)

  + [/memory](#memory)
* [版本信息](#_5)

  + [/version](#version)
* [缓存](#_6)

  + [/cache/fakeip/flush](#cachefakeipflush)
  + [/cache/dns/flush](#cachednsflush)
* [运行配置](#_7)

  + [/configs](#configs)
  + [/configs/geo](#configsgeo)
  + [/restart](#restart)
* [更新](#_8)

  + [/upgrade](#upgrade)
  + [/upgrade/ui](#upgradeui)
  + [/upgrade/geo](#upgradegeo)
* [策略组](#_9)

  + [/group](#group)
  + [/group/group\_name](#groupgroup_name)
  + [/group/group\_name/delay](#groupgroup_namedelay)
* [代理](#_10)

  + [/proxies](#proxies)
  + [/proxies/proxies\_name](#proxiesproxies_name)
  + [/proxies/proxies\_name/delay](#proxiesproxies_namedelay)
* [代理集合](#_11)

  + [/providers/proxies](#providersproxies)
  + [/providers/proxies/providers\_name](#providersproxiesproviders_name)
  + [/providers/proxies/providers\_name/healthcheck](#providersproxiesproviders_namehealthcheck)
  + [/providers/proxies/providers\_name/proxies\_name/healthcheck](#providersproxiesproviders_nameproxies_namehealthcheck)
* [规则](#_12)

  + [/rules](#rules)
  + [/rules/disable](#rulesdisable)
* [规则集合](#_13)

  + [/providers/rules](#providersrules)
  + [/providers/rules/providers\_name](#providersrulesproviders_name)
* [连接](#_14)

  + [/connections](#connections)
  + [/connections/:id](#connectionsid)
* [域名查询](#_15)

  + [/dns/query](#dnsquery)
* [DEBUG](#debug)

  + [/debug/gc](#debuggc)
  + [/debug/pprof](#debugpprof)

# [API](#api)[¶](#api "Permanent link")

## [请求示例](#_1)[¶](#_1 "Permanent link")

curl 示例 `curl -H 'Authorization: Bearer ${secret}' http://${controller-api}/configs?force=true -d '{"path": "", "payload": ""}' -X PUT`

此请求附带 `'Authorization: Bearer ${secret}'` 请求头，其中：

* `${secret}` 为配置文件设置的[api](../config/general/#api)密钥
* `${controller-api}` 为配置文件中设置的[api](../config/general/#api)监听地址
* `?force=true` 为携带参数，部分请求需携带
* `'{"path": "", "payload": ""}'` 为要更新的资源的数据

Note

如果需要传入路径，注意，如果路径不在 mihomo 工作目录，请手动设置`SAFE_PATHS`环境变量将其加入安全路径，该环境变量的语法同本操作系统的 PATH 环境变量解析规则（即 Windows 下以分号分割，其他系统下以冒号分割）。

## [日志](#_2)[¶](#_2 "Permanent link")

### [`/logs`](#logs)[¶](#logs "Permanent link")

获取实时日志

* 请求方法：`GET` / `WS`
* 可选参数：`?level=log_level`, 其中 `log_level` 可选值为 `info`, `warning`, `error`, `debug`

## [流量信息](#_3)[¶](#_3 "Permanent link")

### [`/traffic`](#traffic)[¶](#traffic "Permanent link")

获取实时流量，单位 kbps

* 请求方法：`GET` / `WS`

## [内存信息](#_4)[¶](#_4 "Permanent link")

### [`/memory`](#memory)[¶](#memory "Permanent link")

获取实时内存占用，单位 kb

* 请求方法：`GET` / `WS`

## [版本信息](#_5)[¶](#_5 "Permanent link")

### [`/version`](#version)[¶](#version "Permanent link")

获取 mihomo 版本信息

* 请求方法：`GET`

## [缓存](#_6)[¶](#_6 "Permanent link")

### [`/cache/fakeip/flush`](#cachefakeipflush)[¶](#cachefakeipflush "Permanent link")

清除 fakeip 缓存

* 请求方法：`POST`

### [`/cache/dns/flush`](#cachednsflush)[¶](#cachednsflush "Permanent link")

清除 dns 缓存

* 请求方法：`POST`

## [运行配置](#_7)[¶](#_7 "Permanent link")

### [`/configs`](#configs)[¶](#configs "Permanent link")

获取基本配置

* 请求方法：`GET`

重新加载基本配置

* 请求方法：`PUT`
* 携带参数：`?force=true`

更新基本配置

* 请求方法：`PATCH`
* 携带数据：`'{"mixed-port": 7890}'`

### [`/configs/geo`](#configsgeo)[¶](#configsgeo "Permanent link")

更新 GEO 数据库

* 请求方法：`POST`
* 发送数据：`'{"path": "", "payload": ""}'`

### [`/restart`](#restart)[¶](#restart "Permanent link")

重启内核

* 请求方法：`POST`
* 发送数据：`'{"path": "", "payload": ""}'`

## [更新](#_8)[¶](#_8 "Permanent link")

### [`/upgrade`](#upgrade)[¶](#upgrade "Permanent link")

更新内核

* 请求方法：`POST`
* 发送数据：`'{"path": "", "payload": ""}'`

### [`/upgrade/ui`](#upgradeui)[¶](#upgradeui "Permanent link")

更新面板，须设置 [external-ui](../config/general/#_7)

* 请求方法：`POST`

### [`/upgrade/geo`](#upgradegeo)[¶](#upgradegeo "Permanent link")

更新 GEO 数据库

* 请求方法：`POST`
* 发送数据：`'{"path": "", "payload": ""}'`

## [策略组](#_9)[¶](#_9 "Permanent link")

### [`/group`](#group)[¶](#group "Permanent link")

获取策略组信息

* 请求方法：`GET`

### [`/group/group_name`](#groupgroup_name)[¶](#groupgroup_name "Permanent link")

获取具体的策略组信息

* 请求方法：`GET`

清除自动策略组 fixed 选择

* 请求方法：`DELETE`

### [`/group/group_name/delay`](#groupgroup_namedelay)[¶](#groupgroup_namedelay "Permanent link")

对指定策略组内的节点/策略组进行测试，返回新的延迟信息，并清除自动策略组的 fixed 选择

* 请求方法：`GET`
* 携带参数：`?url=xxx&timeout=5000`

## [代理](#_10)[¶](#_10 "Permanent link")

### [`/proxies`](#proxies)[¶](#proxies "Permanent link")

获取代理信息

* 请求方法：`GET`

### [`/proxies/proxies_name`](#proxiesproxies_name)[¶](#proxiesproxies_name "Permanent link")

获取具体的代理信息

* 请求方法：`GET`

选择特定的代理

* 请求方法：`PUT`
* 携带数据：`'{"name":"日本"}'`

### [`/proxies/proxies_name/delay`](#proxiesproxies_namedelay)[¶](#proxiesproxies_namedelay "Permanent link")

对指定代理进行测试，并返回新的延迟信息

* 请求方法：`GET`
* 携带参数：`?url=xxx&timeout=5000`

## [代理集合](#_11)[¶](#_11 "Permanent link")

### [`/providers/proxies`](#providersproxies)[¶](#providersproxies "Permanent link")

获取所有代理集合的所有信息

* 请求方法：`GET`

### [`/providers/proxies/providers_name`](#providersproxiesproviders_name)[¶](#providersproxiesproviders_name "Permanent link")

获取特定代理集合的信息

* 请求方法：`GET`

更新代理集合

* 请求方法：`PUT`

### [`/providers/proxies/providers_name/healthcheck`](#providersproxiesproviders_namehealthcheck)[¶](#providersproxiesproviders_namehealthcheck "Permanent link")

触发特定代理集合的健康检查

* 请求方法：`GET`

### [`/providers/proxies/providers_name/proxies_name/healthcheck`](#providersproxiesproviders_nameproxies_namehealthcheck)[¶](#providersproxiesproviders_nameproxies_namehealthcheck "Permanent link")

对代理集合内的指定代理进行测试，并返回新的延迟信息

* 请求方法：`GET`
* 携带参数：`?url=xxx&timeout=5000`

## [规则](#_12)[¶](#_12 "Permanent link")

### [`/rules`](#rules)[¶](#rules "Permanent link")

获取规则信息

* 请求方法：`GET`

### [`/rules/disable`](#rulesdisable)[¶](#rulesdisable "Permanent link")

禁用规则，其中 key 为规则的索引，value 为是否禁用该规则，为临时操作，重启后失效

* 请求方法：`PATCH`
* 携带数据：`'{"0": false,"1": true}'`

## [规则集合](#_13)[¶](#_13 "Permanent link")

### [`/providers/rules`](#providersrules)[¶](#providersrules "Permanent link")

获取所有规则集合的所有信息

* 请求方法：`GET`

### [`/providers/rules/providers_name`](#providersrulesproviders_name)[¶](#providersrulesproviders_name "Permanent link")

更新规则集合

* 请求方法：`PUT`

## [连接](#_14)[¶](#_14 "Permanent link")

### [`/connections`](#connections)[¶](#connections "Permanent link")

获取连接信息

* 请求方法：`GET` / `WS`
* 可选参数：`?interval=milliseconds`, 其中 `milliseconds` 为刷新间隔，默认值为 1000 毫秒

关闭所有连接

* 请求方法：`DELETE`

### [`/connections/:id`](#connectionsid)[¶](#connectionsid "Permanent link")

关闭特定连接

* 请求方法：`DELETE`

## [域名查询](#_15)[¶](#_15 "Permanent link")

### [`/dns/query`](#dnsquery)[¶](#dnsquery "Permanent link")

获取指定名称和类型的 DNS 查询数据

* 请求方法：`GET`
* 携带参数：`?name=example.com&type=A`

## [DEBUG](#debug)[¶](#debug "Permanent link")

`/debug` 需要内核启动时 [日志级别](../config/general/#_5) 为 `debug`

### [`/debug/gc`](#debuggc)[¶](#debuggc "Permanent link")

进行主动 GC

* 请求方法：`PUT`

### [`/debug/pprof`](#debugpprof)[¶](#debugpprof "Permanent link")

浏览器打开 `http://${controller-api}/debug/pprof` 可查看原始 DEBUG 信息，其中：

* allocs 表示每个函数调用的内存分配情况，包括在堆栈上和堆上分配的内存大小以及内存分配次数。这个报告主要是为了帮助我们找到代码中存在的内存泄漏、内存频繁申请等问题。
* heap 报告则给出了程序在堆上使用的内存的详细信息，其中包括被分配的内存块的大小、数量和地址，并且按照大小排序。这个报告主要是为了搜寻内存使用过高的地方，我们可以在 heap 报告中查看对象的大小，从而找到内存使用过高的地方。

#### [安装 [Graphviz](https://graphviz.org/download/),可查看图形化的 debug 信息](#graphviz-debug)[¶](#graphviz-debug "Permanent link")

##### [查看图形化 Heap 报告](#heap)[¶](#heap "Permanent link")

```
go tool pprof -http=:8080 http://127.0.0.1:xxxx/debug/pprof/heap
```

[Full image](../assets/image/api/heap.svg)

##### [查看图形化 Allocs 报告](#allocs)[¶](#allocs "Permanent link")

```
go tool pprof -http=:8080 http://127.0.0.1:xxxx/debug/pprof/allocs
```

[示例输出](../assets/image/api/allocs.svg)

##### [提交输出报告](#_16)[¶](#_16 "Permanent link")

浏览器访问 `http://${controller-api}/debug/pprof/heap?raw=true` 即可下载这个文件，通过上传到 [issues](https://github.com/MetaCubeX/mihomo/issues) 提交你遇到的问题。

2026年2月3日

回到页面顶部


Copyright © 2023 mihomo