# AGENTS.md

维护本项目的 AI 编码 agent(Claude Code、Codex 等)指南。
动手前先读。**Critical rules** 里是非显然规则——踩错它们最容易搞坏构建或安全模型。

## 项目概览

原生 macOS 菜单栏应用,展示火山引擎方舟(Ark)**Agent Plan** & **Coding Plan** 用量。技术栈:**Tauri 2 + Rust + React + Vite + TypeScript**,纯 CSS。数据来自本地 `arkcli` SSO 会话——应用 spawn `arkcli usage plan` 解析 JSON。它是 `arkcli` 的一层薄壳,**所有凭证留在 `arkcli`**。

约 940 行 Rust + 1140 行 TS/CSS。非平凡改动前先通读一遍。

## 命令

```bash
npm install                 # node 依赖
npm run tauri:dev           # 开发运行(热重载;首次约 2-4 分钟,增量约 3 秒)
npm run tauri:build         # release 产物 → src-tauri/target/release/bundle/{macos,*.dmg}

# Rust 快速循环(从 src-tauri/ 跑,或带 --manifest-path):
cargo check  --manifest-path src-tauri/Cargo.toml
cargo test   --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt    --manifest-path src-tauri/Cargo.toml
```

`tauri dev` 监听 `src-tauri/src/*.rs` 并重建。**不监听托盘图标 PNG**(见规则 8)。

## 架构

```
src/                         React 浮层(App.tsx + components/ + lib/)
src-tauri/src/
  lib.rs        初始化、Accessory 策略、全局快捷键、刷新循环、invoke 命令
  tray.rs       托盘图标+菜单、浮层定位(tray.rect)、托盘标题计算
  ark_usage.rs  spawn arkcli → 解析 → 归一化(Plan/Period,-1 哨兵,Coding 仅 percent)
  state.rs      缓存快照 + 持久化设置(JSON 在应用配置目录)
docs/技术方案.md            设计 + 选型对比
```

数据流:`spawn arkcli usage plan` → `ark_usage::fetch()` 归一化 → 缓存进 `AppState` → 以 `usage-update` 事件发出 → React 渲染。托盘标题从缓存 + 设置在 Rust 侧重算。

## Critical rules

1. **绝不引入 HTTP/网络客户端。** `Cargo.toml` 没有 `reqwest`/`hyper`/`ureq` 是有意为之——应用自身**零**网络调用,所有数据走 `arkcli`。这是 README 写明的安全特性。如果某功能看似需要 HTTP,改成走 `arkcli`(如 `arkcli usage plan-details`、`arkcli usage stats --mine`)。不要随意加 `tauri-plugin-updater`。

2. **arkcli 默认输出 JSON。** 不要加 `--format json`(冗余,且未来 arkcli 改动可能反而出错)。用固定参数 spawn `arkcli usage plan`——绝不把用户输入拼进子进程参数。`run_arkcli_login` 和 `run_arkcli_install` 也是硬编码 osascript 串,同样规则。

3. **`-1` 是"无数据"哨兵值。** 一律转 `None`(见 `ark_usage.rs` 的 `normalize_period`)。绝不渲染 `-1%`。

4. **Coding Plan 只有 `percent`**(无 `used`/`total`),Agent Plan 两者都有。所有数值字段用 `Option<f64>` 统一建模。

5. **`reset_at` 是 ISO 8601 字符串**(如 `"2026-07-06T00:00:00+08:00"`,带时区偏移),不是 epoch 毫秒。`ark_usage::parse_reset_at` 用 `parse_from_rfc3339` 解析成 epoch 毫秒,失败落 `None`。coding-plan 的 item 还带 `updated_at`(epoch **秒**,整型,目前未用)。

6. **托盘标题在 Rust 侧计算**(`tray::compute_title`),不是前端推送——这样浮层隐藏时标题也会更新。不要加前端 `update_tray_title` 调用路径。标题展示*剩余*%(100 − 已用),按 `Settings` 选定的套餐/周期。

7. **objc2-free 构建。** 用 Tauri 内建 API(`set_always_on_top`、`set_visible_on_all_workspaces`、`tray.rect()`、`set_title`、`set_activation_policy(Accessory)`)。不要无故加 `objc2`/`objc2-app-kit` 依赖;若浮层 z-order 真需要 `NSPopUpMenuWindowLevel`,刻意引入并写明原因(Tokcat 的 `tray.rs` 可参考)。

8. **托盘图标是模板单色 PNG**,经 `include_image!("icons/tray-icon.png")` 在**编译期**嵌入。改完 PNG 要 `touch src-tauri/src/tray.rs` 触发 `tauri dev` 重建——文件监听不跟踪 PNG 变化。源素材 `/tmp/ve_logo.png` 仅用于重新生成;规范来源是 `portal.volccdn.com/obj/volcfe/misc/favicon.png`。保持 `icon_as_template(true)`。

9. **`PlanUsage` 永远返回,绝不返回 Tauri `Err`。** 拉取失败落到三个字段:`not_installed`(`arkcli` 不在 PATH,即 `io::ErrorKind::NotFound`)、`auth_expired`(SSO 失效)、`error`(其它)。前端据此渲染三种引导条(InstallBanner / AuthBanner / 错误条)。`not_installed` 时 `error` 置 `None`(交给 InstallBanner)。新增 fetch 类命令照此模式,不要为预期运行时失败引入 `Result<_, String>`。

10. **设置持久化**到 `app_config_dir/settings.json`,经 `AppState`(纯 JSON 文件,**不是** `tauri-plugin-store`)。新偏好通过 `get_settings`/`set_settings` 包;`set_settings` 已处理 autostart 同步 + 托盘标题重算——扩展它,别绕过。

11. **`arkcli` 经 PATH 解析**(与你 shell 同信任模型)。固定参数 → 无注入。不要加接收用户输入的 shell-out 路径。GUI 启动时进程 PATH 不含 homebrew/nvm 目录,首次 `NotFound` 时 `ark_usage::resolved_shell_path` 用用户登录交互式 shell(`$SHELL -lic`,source `.zprofile`+`.zshrc`)的 PATH 注入重试,`OnceLock` 缓存。`run_arkcli_install` 打开 Terminal 跑硬编码的 `npm install -g @volcengine/ark-cli && arkcli auth login`(系统 Node 可能需用户手动加 `sudo`)。

12. **CSP 当前为 `null`**(已知加固项,在 README 路线图里)。开启严格 CSP 可以,但别破坏本地资源 webview——改完 `tauri.conf.json` 的 `security.csp` 要测浮层。

## 数据源与解析

`arkcli usage plan` → `{ viewer, items:[{product, edition, tier?, updated_at?, periods:[{label, used?, total?, percent, reset_at(string ISO8601)}]}] }`。`updated_at` 仅 coding-plan 有(epoch 秒)。

- Agent 周期:`5h` / `weekly` / `monthly`(有 `used`+`total`)。
- Coding 周期:`session` / `weekly` / `monthly`(仅 `percent`)。
- **tier 是 Agent Plan 专属**:来自 `GetAFPUsage` 响应的 `PlanType` 字段(medium/large/max)。Coding Plan 走 `GetCodingPlanUsage`,响应里**没有** `PlanType`(只有 `QuotaUsage`/`Status`/`UpdateTimestamp`),故 arkcli 不返回 tier——是 API 侧设计,非 arkcli 丢弃。Coding 的"等级"只有 `edition`(personal/team,UI 已显示)。
- `reset_at` 对所有周期都是 **ISO 8601 字符串**(带 `+08:00` 偏移,如 `"2026-07-06T00:00:00+08:00"`),arkcli 已归一掉后端秒/毫秒差异。`ark_usage::parse_reset_at` 用 `parse_from_rfc3339` 解析成 epoch 毫秒,失败落 `None`。
- 认证失效检测是**启发式**(非零退出 / stderr 关键词 `expired|unauthorized|401|login`)。若能抓到真实过期 payload,精修 `UsageError::is_auth_expired`。
- 未安装检测:`Command::new("arkcli")` → `io::ErrorKind::NotFound` → `not_installed=true`。
- 可选进阶数据源(路线图,尚未接):`arkcli usage plan-details --start YYYY-MM-DD`、`arkcli usage stats --mine`。

## 约定

**Rust**
- `///` 文档注释讲*为什么*不讲*是什么*,沿用 `ark_usage.rs`、`tray.rs` 现有风格。
- `#[serde(rename_all = "camelCase")]` 用于发给前端的结构体;`#[serde(skip_serializing_if = "Option::is_none")]` 用于可选字段。
- 共享状态用 `parking_lot::RwLock`;子进程用 `tokio::process::Command`(异步),绝不阻塞运行时。
- UI 文案用中文;后端诊断用 `log::warn!`。

**前端**
- 纯 CSS + CSS 变量在 `src/styles.css`——**不用 Tailwind**,不用 CSS-in-JS。
- Tauri API 经 `isTauri()` 判断;`@tauri-apps/api/...` 在 effect 内动态 `import`(见 `src/lib/runtime.ts`)。
- 每个 `listen()` 和 `addEventListener` 都在 effect 返回里清理。
- 托盘标题**不从**前端推送(见规则 6)。

**设置键** 在 `src/lib/settings.ts`(`DEFAULT_SETTINGS`、`PERIOD_OPTIONS`、`PLAN_OPTIONS`、`INTERVAL_OPTIONS`)和 `src-tauri/src/state.rs`(`Settings`、`period_label_for`)两边各一份。新增选项时两边同步。

## 图片处理

当前 agent 可能不具备图片解析能力(取决于模型)。规则:

- **不要尝试解析或"看"图片。** 即使任务涉及图片,也不要调用图片读取/视觉解析。
- **只整理链接/路径。** 需要整理图片资源时,把图片的链接或文件路径按要求的格式摆好即可,不读取、不描述图片内容。
- **需要视觉信息时主动说明。** 任务确实依赖图片内容(如截图对比、识别图中文字)时,明确告诉用户当前模型不具备该能力,请其改用文字描述,或换支持视觉的模型。

不要因为"试试看能不能读"而消耗精力——直接按上面的规则处理。

## 测试

`cargo test` 覆盖 `ark_usage` 归一化:哨兵转 None、剩余%、真实样本解析、认证失效关键词。新增解析/归一化逻辑**在这里加测试**。前端暂无测试——保持组件足够纯,逻辑尽量下沉到 `src/lib/format.ts`(纯函数)。

## 构建 / 发布说明

- `Cargo.toml [profile.release]`:`lto=true`、`opt-level="s"`、`codegen-units=1`、`strip=true`、`panic="abort"`——保留,保小体积。
- `Cargo.lock` 已提交(可复现构建),不要 gitignore。
- 代码签名/公证**未配置**(路线图)。不要零散加签名配置;要做得作为一个整体改动,连同 `tauri-plugin-updater` 一起。

## v1 范围外(未受命不要加)

趋势图 + 快照缓存、按模型 `plan-details`、动态着色托盘图标、自动更新 + 签名/公证 `.dmg`、Linux/Windows 配置。见 README 路线图。
