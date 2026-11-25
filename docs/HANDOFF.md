# RPlayer 交接说明（v2.2.0 进行中）

> 给下一个会话用。读完这一份就能接着干，不用回看聊天记录。
> 生成时间：2026-11-26，基线 `ebe31a8`。

## 0. 一句话状态

v2 的功能面已经关掉（v2.1.4 已发版），现在做的是 `docs/V2_PLAN.md` 里 **S1–S10 的按稿对齐**：S3 品牌、S1 托盘已完成，**S2 迷你播放器代码完成、证据 2/8**，其余七包未动。

## 1. 硬约束（不遵守会返工）

- 仓库 `D:\Workspace\RPlayer`；契约 `AGENTS.md`。
- 工具链：`RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu`，PATH 加 `C:\Users\Randy\.cargo\bin` 与 `C:\Users\Randy\scoop\apps\mingw\current\bin`，`CARGO_BUILD_JOBS=1`。
- 构建/测试一律 `--release`：`cargo build --release`、`cargo test --release`（恒 71 passed）。
- 版本号只写 `Cargo.toml`，UI 用 `env!("CARGO_PKG_VERSION")`。
- UI 颜色只用 `theme::c()` / `theme::brand::*` / `theme::scene::*`，**不要新增字面量颜色**（N1 已把 46 处清干净）。
- 每个交互控件要有 AccessKit 名称与稳定 Id（`widgets::mark(...)` / `widget_name`）。
- 不要 `cargo fmt --all`（会重排整个仓库）。
- 临时文件只能进 `tools/qa/out/`、`tools/scratch/`（已 gitignore）；测试截图进 `docs/qa/v0.2/` 时会随包被剔除，可以留。

## 2. 已完成（有提交、有证据）

| 包 | 内容 | 提交 |
| --- | --- | --- |
| S3 | Sleeve R 品牌落地：图标 16–1024 + 多尺寸 ico、`logo()` 重画、`theme::brand` 与 `brand.css` 对表、`assets/brand/` 归档 | `0e326c3` |
| S1 | 托盘 D1–D4：280×366、抬头 58 + 2px 只读进度、圆角 8、两条分隔线、35% 禁用行、140ms 淡入上移、三态提示、双击播放暂停 | `2bc4354` |
| S2 代码 | 迷你播放器跟随主题（删掉每帧强制 Forest）、经典态标题 250 上限 | `0ce66f3` |
| S2 证据 | 经典·纸张·有封面、紧凑·夜幕·无封面；删掉一张主题没切成功的错图 | `f887956`、`ebe31a8` |

## 3. 下一步：把 S2 的六态证据补齐（只截图，不改代码）

设置文件：`%APPDATA%\RPlayer\library.json`（字段 `theme_preset` / `light_theme` / `mini_style` / `mini_open` / `current`）。

统一流程（每次都是：改 json → 杀进程 → 启动 → 从托盘菜单开迷你播放器 → 截图）：

```powershell
# 1) 改设置
$j="$env:APPDATA\RPlayer\library.json"; $o=Get-Content $j -Raw | ConvertFrom-Json
$o.theme_preset='forest'; $o.light_theme=$false; $o.mini_style='classic'
($o | ConvertTo-Json -Depth 20) | Set-Content $j -Encoding UTF8

# 2) 重启
Get-Process rplayer -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Process 'D:\Workspace\RPlayer\target\release\rplayer.exe' -WorkingDirectory 'D:\Workspace\RPlayer'

# 3) 用托盘菜单开迷你播放器（顶栏主题按钮的物理点击不稳，别用）
#    PostMessage(trayHwnd, 0x8001, 0, 0x0205) 开菜单；菜单在光标处 280×366；
#    「迷你播放器」行在菜单内 y≈273，x≈+140
& .\tools\qa\phys-click.ps1 -X 1340 -Y 873

# 4) 截图
& .\tools\qa\capture.ps1 -Title 'RPlayer 迷你播放器' -Out 'D:\Workspace\RPlayer\docs\qa\v0.2\v220-S2-mini-<态名>'
```

六个态与建议文件名：
1. `classic-dark`：`theme_preset=forest` + `mini_style=classic`
2. `compact-paper`：`theme_preset=paper` + `mini_style=compact`
3. `long-title`：临时把某首歌 `title` 改长（>30 字），看是否 250px 省略
4. `hover`：鼠标停在迷你播放器窗口内再截一次（顶部四动作应从 hover-only 变 100%）
5. `drag`：`phys-click` 按住标题栏拖 20px，拖到一半截图（虚线态）
6. `empty`：清空队列或把 `current` 置空后打开迷你播放器

补完把 `docs/V2_PLAN.md` 的 S2 段改成 **已完成**，并把六张图写进证据行。

## 4. 之后按这个顺序做（详见 `docs/V2_PLAN.md`）

| 顺序 | 包 | 要点 | 规模 |
| --- | --- | --- | --- |
| 1 | S5 | 歌词编辑 820×430 / 智能歌单 620×430 / EQ 重排 820×250 | 3–4 天 |
| 2 | S4 | 重复歌曲操作台、健康检查三卡点击进明细 | 2–3 天 |
| 3 | S7 | 响应式四档断点（1280 / 1120 / 840 / <840），唯一可能超期的包 | 4–6 天 |
| 4 | S6 | 快捷键/关于对稿、拖入提示、错误态 | 2–3 天 |
| 5 | S8 | 歌词阴影 1px 白边 + 两种壁纸下阴影 | 0.5 天 |
| 6 | S9 | 顶栏 toggle 语义二选一、表格/侧栏补 `WidgetInfo` | 1–2 天 |
| 7 | S10 | **需用户批准**：vendor eframe 给子 viewport 建 AccessKit adapter | 3–5 天 |

顺序可以调；S10 要动依赖，用户没批就跳过它先做 S5。

## 5. 不要重复踩的坑

- **物理点击前必须读实时 `GetWindowRect`**：窗口位置会在 26/52/104/130/156/182/208 之间漂，记死的坐标必错。
- `PrintWindow` 返回的是后台缓冲，同一张图哈希相同不代表窗口冻结。
- `agent-desktop` 的语义点击对真 button 有效，对 `widgets::chip`（role=switch）和表格行无效——这两类要用 `phys-click.ps1`。
- 主窗识别：按 class `Window Class` + 有标题；**不能**取面积最大的窗（会命中 `RPlayerDcompPanel`），也不能只按标题 ` — ` 匹配（无当前曲时标题只有 `RPlayer`）。
- `Ctrl+,` / `Ctrl+T` 这类全局快捷键在有对话框或焦点异常时会失效；稳定的替代路径是**托盘菜单 → 设置**。
- 改 `library.json` 再重启，是造状态最快最稳的办法（主题、迷你样式、当前曲都能这么造）。

## 6. 挂账项（已与用户确认暂缓，别当缺陷修）

D6 无缝播放听感、D9 五态图标、D10 SMTC 系统浮层、E3–E5（逐句编辑/偏移/获取歌词）、C10 与 K5 智能歌单、H1 重复歌曲、H3 CUE 多 FILE。
恢复条件写在 `docs/V2_PLAN.md` §4 与 `docs/qa/V2_REGRESSION.md` 的「暂缓项」表。

另有两条框架级挂账（用户 2026-11-26 明确「先不做」）：子 viewport 无 AccessKit adapter（J6）、首 Tab 焦点停在 window 根节点；两者都要升级 eframe 才能解，已并入 S10。
