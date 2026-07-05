# CLAUDE.md

本项目由 AI agent 维护。**完整指南见 [`AGENTS.md`](AGENTS.md)**——非平凡改动前先读。下面是最高信号的子集,内联以便始终在上下文里。

## 命令

```bash
npm install
npm run tauri:dev            # 运行(首次约 2-4 分钟,增量约 3 秒)
npm run tauri:build          # release 产物
cargo check  --manifest-path src-tauri/Cargo.toml
cargo test   --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## 顶级 Critical rules(完整列表 + 理由见 AGENTS.md)

1. **绝不引入 HTTP/网络客户端。** 所有数据走 `arkcli`——这是写明的安全特性。
2. **arkcli 默认输出 JSON**——不要加 `--format json`。子进程参数固定,绝不拼用户输入。`run_arkcli_login`/`run_arkcli_install` 同样是硬编码 osascript 串。
3. **`-1` 是"无数据"哨兵** → 转 `None`。**Coding Plan 只有 `percent`**(无 used/total)。**`reset_at` 已是毫秒。**
4. **托盘标题在 Rust 侧计算**(`tray::compute_title`),前端不推送。
5. **托盘图标经 `include_image!` 编译期嵌入**——改完 `src-tauri/icons/tray-icon.png` 要 `touch src-tauri/src/tray.rs` 让 `tauri dev` 重建。
6. **`PlanUsage` 永不返回 Tauri `Err`**——失败落到三字段:`not_installed`(arkcli 不在 PATH)/ `auth_expired`(SSO 失效)/ `error`(其它),前端渲染对应引导条。`run_arkcli_install` 打开 Terminal 跑硬编码安装+登录命令。
7. **objc2-free 构建**——用 Tauri 内建(`set_always_on_top`、`tray.rect()`、`set_title`)。设置经 `AppState` JSON 持久化(不是 `tauri-plugin-store`)。

## 提交前

`cargo fmt` + `cargo clippy -- -D warnings`,`npm audit` 保 0,不要 gitignore `Cargo.lock`,签名/updater 配置不要零散加。
