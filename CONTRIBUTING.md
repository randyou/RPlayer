# 参与 RPlayer

RPlayer 目前只支持 Windows x64，界面基于 Rust + egui。欢迎提交 issue 和 pull request。

## 本地开发

```powershell
git clone https://github.com/randyou/RPlayer.git
cd RPlayer
cargo build --release --bin rplayer
cargo test --release
```

运行 `target\release\rplayer.exe` 即可。SMTC 助手是独立 crate：

```powershell
cd tools/smtc-helper
cargo build --release
```

## 约定

- 版本号只写在 `Cargo.toml`，界面通过 `env!("CARGO_PKG_VERSION")` 读取。
- 颜色只能来自 `theme::c()` / `theme::brand` / `theme::scene`，不要新增颜色字面量。
- 每个可交互控件都要有 AccessKit 名称和稳定 Id。
- 曲目数据存 `library.db`，设置与队列存 `library.json`，写入使用「临时文件 → rename」并保留 `.bak`。
- 临时文件只放进 `tools/qa/out/`、`tools/scratch/`（两者都不入库）。
- 风格细节见 [AGENTS.md](AGENTS.md)；设计稿在 `docs/qa/v0.2/`。

## 提交 PR 前

1. `cargo test --release` 全绿；
2. 改到界面时附上一张改动前后的截图；
3. 一次 PR 只做一件事，方便审查和回滚。
