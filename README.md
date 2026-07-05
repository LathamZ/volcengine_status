# Volcengine Status

> 原生 macOS 菜单栏应用,直接读取本地 `arkcli` SSO 会话,展示火山引擎方舟(Ark)**Agent Plan** 与 **Coding Plan** 的用量。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

面向火山引擎方舟大模型平台开发者的长期开源小工具。v1 实时展示套餐用量(5h / weekly / monthly),带阈值变色进度条、重置倒计时和可配置的菜单栏标题——全部由你现有的 `arkcli` SSO 会话驱动。**无需管理 API Key,应用自身不发起任何网络请求,无遥测。**

## 功能

- **菜单栏标题**展示 Agent / Coding 套餐的*剩余*百分比(如 `A 76%  C 99%`),展示哪些套餐、取哪个周期(短周期 / weekly / monthly)均可在设置里切换。
- **浮层**展示各周期进度条:
  - Agent Plan — 5h / weekly / monthly,带 `used / total`。
  - Coding Plan — session / weekly / monthly(仅百分比)。
- **阈值变色**(绿 / 琥珀 / 红),阈值可配。
- **实时重置倒计时**(`3h 20m 后`),每 30 秒刷新。
- **自动刷新**(默认 5 分钟,可配)+ 手动刷新(`⌘R`)。
- **SSO 失效引导条**,一键"登录"打开 Terminal 跑 `arkcli auth login volc-sso`。
- **arkcli 未安装引导条**,检测不到 `arkcli` 时提示安装命令,一键"在终端安装"。
- **菜单栏配件**(Menu Bar Accessory)——隐藏 Dock 图标;跟随系统深色模式;可选开机自启。
- 全局唤出快捷键:`Ctrl+⌘+V`。

## 工作原理

```
┌── 菜单栏 ───────────────────────────────────────────────────┐
│  [图标]  A 76%  C 99%   ← 标题在 Rust 侧计算                    │
└───────────────┬─────────────────────────────────────────────┘
                │ 点击 / Ctrl+⌘+V
                ▼
┌── 浮层(Tauri webview,侧栏毛玻璃)──────────────────────────┐
│  HeaderBar · PlanCard(Agent) · PlanCard(Coding) · footer        │
│  ↑ invoke('get_usage') / listen('usage-update')                  │
└───────────────┬─────────────────────────────────────────────┘
                │ Tauri 命令
┌───────────────┴── Rust 后端 ────────────────────────────────┐
│  lib.rs       初始化 · 刷新循环 · 快捷键 · 命令               │
│  tray.rs      托盘图标 + 浮层定位 + 标题计算                    │
│  ark_usage.rs spawn arkcli → 解析 → 归一化                     │
│  state.rs     缓存 + 持久化设置                                  │
└───────────────┬─────────────────────────────────────────────┘
                │ tokio::process::Command(固定参数,无 shell)
                ▼
           arkcli  ──SSO──▶  火山引擎方舟
```

应用是 `arkcli` 的一层薄壳:spawn `arkcli usage plan` → 解析 JSON → 归一化两种套餐 → 渲染。所有凭证都留在 `arkcli` 侧。

## 前置条件

- **macOS 11+**(Linux/Windows 经 Tauri 可跨平台,但尚未配置)。
- **Rust**([rustup](https://rustup.rs))+ **Node 20+**。
- **`arkcli`** —— 火山引擎方舟 CLI(npm 包 [`@volcengine/ark-cli`](https://www.npmjs.com/package/@volcengine/ark-cli)):
  ```bash
  npm install -g @volcengine/ark-cli   # 安装
  arkcli auth login volc-sso          # SSO 登录(打开浏览器)
  arkcli usage plan                    # 验证——应输出 JSON
  ```
  如果 `arkcli` 不在 `PATH` 上,浮层会弹出**安装引导条**,显示上述命令并提供"在终端安装"按钮,一键打开 Terminal 执行。(nvm 管理的 Node 无需 `sudo`;系统 Node 可能需要手动加 `sudo`。)

## 从源码构建

暂无预编译二进制,从源码构建:

```bash
git clone https://github.com/LathamZ/volcengine_status.git
cd volcengine_status
npm install
npm run tauri:dev        # 开发模式运行(热重载;首次构建约 2-4 分钟,增量约 3 秒)
npm run tauri:build     # 产出 src-tauri/target/release/bundle/{macos,*.dmg}
```

## 使用

| 操作 | 快捷键 |
|---|---|
| 切换浮层 | `Ctrl+⌘+V`(全局)或点击托盘图标 |
| 刷新 | `⌘R` |
| 设置 | `⌘,` |
| 关闭浮层 | `Esc` / `⌘W` |
| 退出 | `⌘Q`(托盘菜单) |

设置(齿轮图标)可调:刷新间隔、托盘标题展示哪些套餐与哪个周期、告警/严重阈值、开机自启。

## 配置

设置持久化到 `~/Library/Application Support/com.lathamzhao.volcenginestatus/settings.json`(macOS 应用配置目录)。应用内可改,不存任何密钥。

| 键 | 类型 | 默认 | 说明 |
|---|---|---|---|
| `refreshIntervalSecs` | number | 300 | 下限 30 秒 |
| `trayPlans` | string[] | `["agent-plan","coding-plan"]` | 托盘展示的套餐(子集/顺序) |
| `trayPeriod` | `"short"`/`"weekly"`/`"monthly"` | `"monthly"` | `short` = 5h(Agent)/ session(Coding) |
| `thresholdWarn` | number | 70 | 百分比,达到变琥珀 |
| `thresholdCritical` | number | 90 | 百分比,达到变红 |
| `autostart` | boolean | false | 开机自启(LaunchAgent) |

## 安全性

开源且体积小,易于审计(约 940 行 Rust + 1140 行 TS/CSS)。威胁模型与属性如下:

**凭证从不触碰。** 应用不读取、不存储、不传输 `arkcli` 的 SSO token,只 spawn `arkcli usage plan`,由 arkcli 用自己的会话。应用内无需配置任何 API Key。✅

**自身零网络调用。** `Cargo.toml` 不含任何 HTTP 客户端(`reqwest`/`hyper`/`ureq`…,已核实)。应用自身**不发起任何**对外网络请求,唯一网络流量来自 `arkcli` 自身,走你已信任的会话。无遥测、无分析、无自动更新回连。✅

**无子进程注入。** `arkcli usage plan`、登录助手、安装助手(`osascript → Terminal → npm install -g @volcengine/ark-cli && arkcli auth login volc-sso`)都用**硬编码参数串**,无用户输入插值 → 无命令注入。⚠️ `arkcli` 经 `PATH` 解析(与你 shell 同信任模型),若 PATH 中靠前的位置有恶意同名二进制会被执行。想加固可在启动时解析 `arkcli` 的绝对路径(欢迎 PR)。

**CSP 关闭。** `tauri.conf.json` 设 `"csp": null`。webview 只加载本地打包资源、不渲染远程或用户提供的 HTML,实际风险低——但建议开启严格 CSP 做纵深防御。⚠️

**最小文件访问。** 应用只写 `settings.json`(纯偏好,无密钥)到 macOS 应用配置目录,默认用户级权限。不碰剪贴板、相机、麦克风、定位。✅

**错误信息留在本地。** `arkcli` stderr(裁剪后)可能出现在应用内错误/认证条;绝不写盘或外发。`arkcli usage` 错误不含 token。✅

**依赖卫生。** 依赖树很小且都是知名 crate:`tauri 2`、`serde`、`serde_json`、`tokio`、`parking_lot`、`chrono`、`log`、`env_logger`、`tauri-plugin-{autostart,global-shortcut}`。`Cargo.lock` 已提交(可复现构建)。`npm audit` → **0 漏洞**。⚠️ 建议在 CI 里跑 `cargo audit`(尚未接入)以扫 Rust advisory。

**代码签名/公证:未配置。** 预编译 `.dmg` 未签名(路线图)。可信使用请从源码构建;未签名预编译二进制会被 macOS Gatekeeper 拦截。⚠️

**`macOSPrivateApi` / 透明窗口。** 侧栏毛玻璃效果所需;不授予额外能力。是说明而非漏洞。

安全问题请通过 GitHub 私密漏洞报告或邮件联系维护者。

## 性能

**刷新模型。** 默认 5 分钟一次(下限 30 秒)spawn `arkcli usage plan`。`arkcli` 是 Node CLI(冷启动约 100-300ms)+ 一次网络往返,5 分钟周期下可忽略。手动刷新绕过缓存。

**不忙等。** 刷新循环 sleep 在 `tokio` 定时器上。浮层隐藏时 webview 常驻(状态保留),只做 30 秒倒计时 + 5 分钟拉取,无空转。

**异步不阻塞。** `arkcli` 用 `tokio::process::Command` 异步 spawn;小 JSON 解析无需 `spawn_blocking`,Tauri 事件循环从不阻塞。

**开浮层即出。** 最近快照缓存在 `AppState`(`parking_lot::RwLock`)里,`get_usage` 直接返回缓存,开浮层零等待;托盘标题从缓存 O(plans×periods) 重算。

**前端精简。** `ResizeObserver` 经 `requestAnimationFrame` 防抖;监听器都清理;唯一定时器是 30 秒倒计时 `setInterval`。无轮询循环。

**小内存占用。** 缓存负载仅几 KB。Tauri 菜单栏应用常驻内存约 40-80MB。v1 不保留历史(趋势快照见路线图)。

**release 二进制优化。** `Cargo.toml [profile.release]`:`lto=true`、`opt-level="s"`、`codegen-units=1`、`strip=true`、`panic="abort"`,最小化体积。用 `npm run tauri:build` 构建。

## 架构

```
src/                        React 浮层
  App.tsx                   状态、事件、快捷键、窗口高度自适应
  components/               HeaderBar · PlanCard · PeriodRow · AuthBanner · InstallBanner · SettingsPanel
  lib/                      types · format · settings · runtime(invoke/listen 封装)
  styles.css                纯 CSS + CSS 变量,跟随系统深色
src-tauri/src/
  lib.rs        初始化、Accessory 策略、全局快捷键、刷新循环、命令
  tray.rs       托盘图标+菜单、浮层定位(tray.rect)、标题计算
  ark_usage.rs  spawn arkcli、解析、归一化(-1 哨兵、Coding 仅 percent)
  state.rs      缓存快照 + 持久化设置(JSON 在应用配置目录)
docs/技术方案.md            设计 + 选型对比
```

托盘标题在 **Rust 侧**从缓存快照 + 设置计算,所以浮层隐藏时也会更新。

## 数据源与踩坑

`arkcli usage plan`(默认 JSON——`--format json` 冗余)。已处理的边界:

- `-1` 是哨兵值 → 渲染为 `—`。
- Coding Plan 周期只有 `percent`(无 `used`/`total`)。
- `reset_at` 是 epoch 毫秒(arkcli 已把 Coding 的秒 ×1000 归一)。
- 非零退出 / 非法 JSON / 认证失效关键词 → 认证条;arkcli 不在 PATH → 安装条。

## 路线图

- [ ] 本地快照缓存 + 趋势小图
- [ ] 按模型明细(`arkcli usage plan-details`)
- [ ] 动态着色托盘图标
- [ ] CI 接入 `cargo audit` + clippy
- [ ] 严格 CSP
- [ ] 签名 + 公证的 `.dmg` + `tauri-plugin-updater`
- [ ] Linux / Windows 配置

## 贡献

欢迎 PR。请:

- 提交前跑 `cargo fmt` + `cargo clippy -- -D warnings`。
- 保持依赖树精简("零 HTTP 客户端"是特性,无强理由别加网络客户端。
- 跑 `npm audit`,目标 0 漏洞。
- 为 `ark_usage.rs` 的归一化逻辑加测试(已有起步单测)。

## 许可证

[MIT](LICENSE) © LathamZhao。如需 Apache-2.0 等其它许可证可替换。

## 致谢

- [Tokcat](https://github.com/handlecusion/tokcat)——同栈(Tauri 2 + Rust),菜单栏托盘与浮层模式参考自它。
- [火山引擎 `arkcli`](https://www.volcengine.com/)——本应用封装的数据源,SSO 认证。
- [Tauri](https://tauri.app/)——跨平台应用框架。
