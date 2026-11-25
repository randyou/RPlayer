# RPlayer v2 后续开发计划（设计稿 + 品牌标识已补齐 · v2.1.4 之后）

> 基线：`v2.1.4`（tag `v2.1.0`–`v2.1.4`），A–O 十五组回归已完成，9 项缺陷已修，R2 定性为工具限制。
> 输入：`docs/qa/v0.2/DESIGN_SPEC_GAPS_SUPPLEMENT.md`（D1–D15）、`docs/qa/v0.2/spec2-*.png`（六张补充稿）、`design/rplayer-v2/brand/`（Sleeve R 品牌资产）、`docs/V2_REMAINING.md`、`docs/qa/V2_REGRESSION.md`（含末尾「暂缓项」表）。

## 0. 结论先行

v2 的功能面已经关掉，剩下的两件事性质完全不同：

1. **按稿重做**（约占 15–22 人日）：功能都在，缺的是尺寸、状态、文案、动效与品牌。这是「对齐」，不是「开发」。
2. **框架升级**（约占 3–5 人日）：子窗口的 AccessKit 适配器与首个 Tab 焦点，卡在 eframe 上游，需要 vendor 或升级。

另外有 8 类验收项只能靠人耳/人眼/真实 fixture（D6、D9、D10、E3–E5、C10、K5、H1、H3），已登记为「暂缓」，恢复条件写在本文 §4。

**建议节奏**：v2.2.0 一次做完「按稿对齐 + 可访问性框架」，随后 v2.3.0 做规模化。中间不再插小版本，除非出现阻塞性缺陷。

---

## 1. v2.1.x 已完成的部分（供对照，不再重复）

| 类别 | 内容 |
| --- | --- |
| 回归 | A–O 十五组约 90 项，逐项判据/结果/证据落在 `docs/qa/V2_REGRESSION.md` |
| 缺陷修复（9） | 右键菜单可点（拖拽载荷吞点击）、侧栏底栏被播放条压住、锁定歌词被原生拖动绕过、专辑/艺术家视图队列入口失效、播放模式按状态命名、进度/音量滑条公布数值、桌面歌词右键菜单（设置/锁定/关闭）、46 处硬编码色收敛成 `theme.rs` token、自绘窗口控制按钮命名 |
| 定性未修（2） | R2（辅助技术激活）＝自动化工具限制；J6（子窗口 AccessKit）＋首 Tab 焦点＝eframe 上游缺口，转入 v2.2.0 |
| 性能 | 10k 曲库滚动 p50 2.0ms / p95 2.5ms、内存 205MB、空闲 CPU 1.72% |
| 发布 | v2.1.0 → v2.1.4，五个 tag，`cargo test --release` 71 passed |

---

## 2. v2.2.0 按稿对齐（P0，15–22 人日）

### S1 托盘菜单 D1–D4（S，1–2 天）· **已完成**

- 抬头 58px（封面 36 + 曲名/艺人 + `已播 / 总时长`），抬头下沿加 2px 只读进度条（不响应点击，避免误拖进度）；
- 圆角 8、行高 30、九个 30px 行、**只两条分隔线**（窗口与传输 / 收藏与浮层 / 设置与退出，`设置` 与 `退出` 同组）；
- 无曲目时传输与收藏行保留位置、按 35% 不透明度置灰（`src/ui/menu.rs` 的 `item_impl` 统一处理，行网格不跳动）；
- 弹出动画从「缩放 0.94→1.0」改为「140ms 淡入 + 上移 2px」（`ui.multiply_opacity` + `TSTransform` 平移）；
- 托盘提示改为三态：`RPlayer · 正在播放 · 曲名 — 艺人` / `RPlayer · 已暂停 · 曲名` / `RPlayer · 未在播放`，经 `set_tray_tooltip()` 写入并 `NIM_MODIFY` 刷新；
- 双击托盘图标 = 播放/暂停（`WM_LBUTTONDBLCLK → TrayAction::TogglePlay`），单击仍是显示/聚焦主窗口；
- 图标不随状态换脸（延续 S3 的 `assets/icon.ico`，16/24/32 三档取自同一 ICO）。

**证据**：`docs/qa/v0.2/v220-S1-tray.png`（280×366，实测尺寸与稿一致）；`cargo test --release` 71 passed。待人工：悬停 500ms 看三态提示、双击图标试播放/暂停。

### S2 迷你播放器 D15（S，1–2 天）· **已完成**

已改（`src/overlay.rs`）：
- 不再强制 `ThemePreset::Forest`：迷你播放器跟随应用主题，经典 / 紧凑 × 纸张 / 夜幕 × 有 / 无封面八态都能出现；
- 顶部四动作改成稿注的悬停语义：静止 55% → 指针进入后 140ms 升到 100%（`ctx.animate_bool_with_time` + `ui.multiply_opacity`）；
- 拖动反馈：按住封面或标题条拖动时，沿圆角 16 采样闭合路径画 1px accent 虚线（`Shape::dashed_line`，4/3 段距），松手走 `snap_and_store_mini` 把窗口位置写回 `library.json`；
- 标题单行 + 250px 上限：紧凑态按稿 250；经典态扣掉 92 封面与 4×24 动作条后只剩约 152px，取 `min(250, available - 128)`，长标题在动作条之前省略，不再压住图钉与「词」按钮；
- 迷你标题 / 副行改 `selectable(false)`：文字选择会吞掉标题条拖动（实测首版拖动只选中了标题文字，窗口没动）；
- 空态：标题 `RPlayer 迷你播放器`、副行 `从音乐库选择一首歌`，经典态仍留 92 音符占位，不出现「未找到封面」。

证据（`docs/qa/v0.2/`，均实测 410×146）：

| 态 | 文件（`-screen.png`，另有同内容 `-print.png`） |
| --- | --- |
| 经典·纸张·有封面 | `v220-S2-mini-classic-paper-art-screen.png` |
| 经典·夜幕·有封面 | `v220-S2-mini-classic-forest-art-screen.png` |
| 经典·纸张·无封面 | `v220-S2-mini-classic-paper-noart-screen.png` |
| 经典·夜幕·无封面 | `v220-S2-mini-classic-forest-noart-screen.png` |
| 紧凑·纸张·无封面 | `v220-S2-mini-compact-paper-noart-screen.png` |
| 紧凑·夜幕·无封面 | `v220-S2-mini-compact-forest-noart-screen.png` |
| 经典·长标题省略 | `v220-S2-mini-classic-paper-longtitle-screen.png` |
| 紧凑·长标题省略 | `v220-S2-mini-compact-paper-longtitle-screen.png` |
| 悬停（动作组 100%） | `v220-S2-mini-classic-paper-art-hover-screen.png` |
| 拖动（1px 虚线） | `v220-S2-mini-classic-paper-art-drag-screen.png` |
| 空态（经典） | `v220-S2-mini-classic-paper-empty-screen.png` |
| 空态（紧凑） | `v220-S2-mini-compact-paper-empty-screen.png` |

取证脚本：`tools/qa/mini-state.ps1`（改 session/library.json → 重启 → 点播放条迷你按钮 → 截图，`-Hover` 把指针停进卡内）、`tools/qa/phys-drag.ps1`（按住拖 60×40 后按住截图再松手）。

未做（记录，不阻塞 D15 收口）：
- 拖动时整窗 0.96 不透明度 + 外部阴影：`ViewportCommand::StartDrag` 会进入系统模态移动循环，期间不重绘，无法逐帧改窗口透明度；阴影由系统绘制。1px 虚线是拖动中唯一的应用侧反馈。
- 280ms 进入动画未做（稿的动效标注，当前窗口即时出现）。
- 紧凑态长标题按稿 250 上限省略，末尾字符会压到 55% 的图钉；稿的 `COMPACT · ELLIPSIS / 250 MAX` 样张同样如此，未额外加避让。

### S3 品牌落地（S，1 天）· **已完成**

- 窗口 / 任务栏 / 托盘图标：`tools/qa/gen-brand-icons.py` 按 `rplayer-mark.svg` 的 64 单位网格用 Pillow 生成 1024/256/48/32/24/16 与多尺寸 `assets/icon.ico`；`build.rs` 仍用 `winresource` 嵌入，托盘经 `ExtractIconExW` 取同一资源，换 ico 即三处同步。
- 应用内 logo：`src/ui/icons.rs` 的 `logo()` 从「字体 R + 三竖条」改为品牌 mark（forest 底 + paper 竖干/碗形/内孔/腿），侧栏头部、关于对话框、空态页三处自动跟随，深底直接用 mark（符合「深底必须反白」）。
- 品牌资产归档到 `assets/brand/`（favicon / apple-touch / og / brand-sheet / 四个 lockup SVG / mark SVG / brand.css / 品牌 PDF），`docs/RELEASE_CHECKLIST.md` 的打包步骤新增 `assets/brand → dist/RPlayer/brand`。
- `theme::brand` 与 `brand.css` 逐项对表：`INK 1D1B16`、`INK_2 37332B`、`MUTED 6B6457`、`FAINT 938975`、`PAPER F4EFE5`、`PAPER_HI FAF6ED`、`PAPER_SUNK EFE8DB`、`FOREST 2E5B44`、`FOREST_DEEP 254A37`、`SIGNAL B64B31`、`NOCTURNE 0F120F`；原 `INK=0F120F` 的语义错位已修正（那其实是 `--nocturne`），另立 `ON_DARK_MUTED B0A594` 给 nocturne 上的次级文字。

**验收**：`docs/qa/v0.2/v220-S3-sidebar.png`、`v220-S3-about.png` 与 `brand-sheet.png` 并排核对；`assets/brand/brand-sheet.png` 的禁用项（不加投影、不用渐变、16px 以下不描边）逐条确认已满足。

### S4 曲库工具 D5–D6（M，2–3 天）· **已完成**

已改（`src/app/health.rs`、`src/app/dialogs.rs`、`src/ui/icons.rs`、`src/app/mod.rs`）：

- **曲库健康检查** 720×340（内容 688×308、边距 16）：LIBRARY HEALTH 眉标 + 曲库健康检查 serif 26 + 副行「已扫描 N 首 · M 项需要关注」；三张数字卡（serif 30，>0 用 `theme::signal()` 橙红）同底板 + 1px 竖分隔，卡可点进明细、再点同一张回汇总（`health_pane: Option<HealthPane>`），选中态 `accent_alpha(0.08)` 底 + 1.5px accent 下划线；问题行 44 高（图标 15 / 标题 12.5 / 副行 9.5 / 右侧 64×28 描边按钮，每行只给一个动作）；汇总态三行（缺失文件→定位、空标签→去编辑、重复候选→查看），明细态逐条列出；空态 Check 24 accent + 「没有发现明显问题」+「已扫描 N 首，缺失文件 0 · 空标签 0 · 重复候选 0。」；底栏「检查不会删除音频，也不会自动删除文件。」+ 重新检查。重新检查在后台线程跑 `crate::health::scan`，窗口保留并显示「正在重新检查…」。
- **重复歌曲** 720×340（Modal 内边距 20，内容 680 宽）：DUPLICATES 眉标 + 重复歌曲 serif 26 + 副行「N 组 · M 个文件候选 · 同一标题、艺术家且时长相差不超过 2 秒」；组卡（card 底 + border_soft + 圆角 10）组头 34 高（「标题 — 艺术家」+ 右侧「N 个文件」），文件行 42 高（保留行 `accent_alpha(0.08)` 底 + 2px accent 左条、文件名 mono 10.5、副行「格式 · 时长 · 目录」mono 8、右侧「保留/忽略」，整行可点即选它作保留），卡脚「仅保留此首」96×28 / 定位 / 忽略本组；对话框底栏「只删除重复的文件记录，其余移出曲库，不删除磁盘文件。」+ 取消 + danger「删除 N 个重复项」。
- **保留候选** `duplicate_keep`：无损优先（flac/wav/aiff/alac）→ `added_at` 最早 → 路径最短；组内文件行按「保留在前」排序，避免误点。
- **图标**（`src/ui/icons.rs`）：新增 `Warning`（三角 + 竖线 + 点）与 `Duplicate`（两个错位圆角矩形），用于问题行。
- 删除仍走 `Outcome::KeepDuplicate` → `keep_duplicate_track`（rfd 原生确认后从曲库移除组内其它项，不删除磁盘文件）；「忽略本组」走 `Outcome::IgnoreDuplicates`。

证据（`docs/qa/v0.2/`，均实测 720×340，另有同内容 `-print.png`）：

| 面 | 文件（`-screen.png`） |
| --- | --- |
| 曲库健康检查 · 空态 | `v220-S4-health-empty-screen.png` |
| 曲库健康检查 · 有问题（1 组重复候选） | `v220-S4-health-issues-screen.png` |
| 曲库健康检查 · 三卡进明细（重复候选） | `v220-S4-health-drill-screen.png` |
| 曲库健康检查 · 缺失文件（1 首） | `v220-S4-health-missing-screen.png` |
| 重复歌曲 · 操作台（保留/忽略 + danger 删除） | `v220-S4-duplicates-screen.png` |

取舍（记录，不阻塞 D5–D6 收口）：
- 「忽略本组」放在组卡脚而不是对话框底栏：多组时每组独立操作更直接，单组时视觉等价于稿面。
- 文件行取 42 而非稿面 36：需要两行文本（文件名 + 格式/时长/目录），36 会裁切。
- 健康检查明细态沿用 44px 问题行，不引入第二套行高。
- 删除确认仍用系统原生对话框；若 S10 统一改成内联确认，再随该包调整。

### S5 编辑类对话框 D7–D9（M，3–4 天）· **已完成**

已改（`src/app/lyrics_edit.rs`、`src/app/smart.rs`、`src/app/equalizer.rs`，配套 `src/lyrics.rs`、`src/theme.rs`、`src/ui/widgets.rs`）：

- **歌词编辑器** 820×430（`fixed_size(788×398)` + 16px 边距，egui 的 `fixed_size` 只算内容区）：LYRICS EDITOR 眉标 + 编辑歌词 serif 28 + 副行「曲名 · 艺人 · 本地编辑」；表头 时间/歌词 + 发丝线；行高 34、时间列 62、歌词起点 96；当前行 `accent_alpha(0.10)` 底 + 2px accent 左条；行尾播放 22（从该句起播）与删除 22；「+ 新增一句」固定在列表外，避免滚动条裁切；底栏「TIME 00:00.00 / 空文本 = 纯文本行」+ 取消 / 保存歌词。
- **歌词数据层**（`src/lyrics.rs`）：`LyricsEditor` 增加 `artist` 与 `baseline`，新增 `is_dirty` / `invalid_row` / `any_invalid` / `first_invalid_row` / `current_index(position)`；时间统一 `format_timestamp_plain`（`mm:ss.xx`，编辑器内不带方括号）。时间非法时该行标 danger 红、右上出现「未保存」chip、保存按钮禁用，不弹窗。
- **智能歌单** 620×430：固定三段列宽 字段 116 / 操作 88 / 值 flex（`allocate_ui_with_layout` 锁宽，避免随文字跳列）；行高 30、行距 4；序号 `01`/`02` mono faint；空值行 danger 红框 + 底部只留一条「第 N 条规则缺少值」，保存禁用；`+ 添加条件` 虚线圆角按钮（`Shape::dashed_line` 手绘）；PREVIEW 块 54 高（sage 底 + 2px accent 左条 + 「N 首匹配」serif 15）；名称输入与值输入用 Frame(card + 1px border + 圆角 8) 包无框 `TextEdit`；ComboBox 下拉用 `ctx.set_style` 临时换 paper 面与 accent 选中（egui 的 popup 只读 `ctx.style()`，改 `ui.style` 无效），高度放到 240 让 8 个字段可滚。
- **空值语义**：`rule_matches` 里空值直接返回 false（此前 `包含 ""` 会匹配全库，导致预览永远不是 0 首）；`can_save = 规则非空 && 名称非空 && 无缺失值`。
- **均衡器** 820×250：`theme::scoped(&theme::DARK, …)` 让整窗固定 Nocturne（新增线程局部调色板覆盖，可嵌套，退出即恢复，纸张主题下也是暗面）；面板内容 788×218、边距 16；头部 EQUALIZER / 均衡器 serif 24 / 「10 段 · 仅作用于当前输出」+ 已启用 toggle + 关闭；预设 8 个 pill 单行横向滚动（`ScrollBarVisibility::AlwaysHidden`，不折行）；10 根推子 62px 间距、82 高、1.5px 细轨 + 空心 accent 描边 knob，双击归零、上下键 ±0.5；底栏 PREAMP 可拖字段（0.1 dB/px）+「修改即时生效，不写入音频文件」+ 重置（保留 enabled）。
- **共用控件**（`src/ui/widgets.rs`）：新增 `pill`（26 高、99 圆角、选中实心 accent）、`primary_button_compact` / `secondary_button_compact`（32 高、圆角 8、禁用走 border_soft + faint）、`toggle_switch_named`（可传 AccessKit 名称）。

证据（`docs/qa/v0.2/`，均实测尺寸，另有同内容 `-print.png`）：

| 面 | 文件（`-screen.png`） |
| --- | --- |
| 歌词编辑器 · 正常 | `v220-S5-lyrics-editor-screen.png`（820×430） |
| 歌词编辑器 · 非法时间 + 未保存 | `v220-S5-lyrics-invalid-screen.png` |
| 智能歌单 · 构建器 | `v220-S5-smart-builder-screen.png`（620×430） |
| 智能歌单 · 空值禁用保存 | `v220-S5-smart-incomplete-screen.png` |
| 均衡器 · 纸张主题下的 Nocturne 面板 | `v220-S5-equalizer-screen.png`（820×250） |

取舍（记录，不阻塞 D7–D9 收口）：
- 歌词行高取 34 而非稿面文字写的 36：430 高内 7 行 × 36 + 新增一句 + 表头 + 底栏会溢出，34 刚好排满且行距仍明显。
- 智能歌单规则行取 30：稿面实测 27、稿注「共用 36px 行」，两者都不能同时满足 3 行 + 添加 + 预览 + 底栏，取中值。
- 保存按钮在未脏时也可点（写回同样内容）；未加「脏才可保存」门控，避免用户误以为不能保存。
- 均衡器的调色板覆盖只管自绘与共用控件；egui 原生弹层（如未来加入文本输入）仍随全局主题，当前 EQ 内无文本输入，不触发。

### S6 系统面 D10–D12（M，2–3 天）

快捷键 820×460（21 条两栏，已有，需对稿）、关于 380×280（已有，需对稿）、拖入提示 380×220、错误态文案。

### S7 响应式骨架 D13（L，4–6 天）· **已完成**

已改（新增 `src/app/layout.rs`，配套 `src/app/sidebar.rs`、`src/app/player_bar.rs`、`src/app/topbar.rs`、`src/app/now_rail.rs`、`src/app/library_view.rs`、`src/app/health.rs`、`src/ui/widgets.rs`、`src/ui/components.rs`、`src/ui/tokens.rs`、`src/main.rs`）：

- **断点模块** `layout::Tier { Wide, Medium, Compact, Narrow }`：≥1280 / ≥1120 / ≥840 / 其余；`MIN_WIDTH 720`、`MIN_HEIGHT 600`；派生 `sidebar_width`（236/236/72/0）、`player_height`（92/92/76/96）、`shows_rail`（仅 Wide）、`shows_album_column`（Wide|Medium）、`icons_only_sidebar`、`drawer`、`dialog_width(viewport, preferred)`（Narrow 下 `min(92vw,720)`）。每帧按逻辑宽度求值，断点切换不做动画、直接重排。
- **侧栏**：Wide/Medium 仍是 236 带文案；Compact 收成 72 宽图标轨（新 `widgets::nav_icon_item`，40 高、17px 图标、tooltip + AccessKit 名，选中态沿用左 2px accent 条）；Narrow 变抽屉——`Area` 前景层 + 半透明遮罩，宽 236、满高、可滚动，点遮罩 / Esc / 选中任一导航即关（`sidebar_drawer_open`）。
- **顶栏**：Narrow 档在面包屑左侧插入「打开导航」按钮；面包屑拖动层起点随之后移 36px，避免拖动层吃掉按钮点击。
- **右栏**：`rail_available` 改为 `Tier::shows_rail()`（≥1280），1120–1279 不再尝试显示。
- **曲库表**：`Columns::for_width_opt(width, show_album)`，Compact/Narrow 收起专辑列（表头与行内文本都不画），列变为 `# / 曲目+艺人 / 时长`；页头标题在 <1120 从 serif 42 降到 32（`library_view` 与 `components::page_title` 两处）。
- **播放条**：92 / 76 / 96 三档。Compact 用 40px 播放键 + 26px 侧键、侧栏列 220；Narrow 改两行——第一行 44% 封面信息 + 运输簇，第二行进度条独占整行，右侧 extras 隐藏（音量 / 歌词 / EQ / 迷你播放器仍在 Wide、Medium 保留）。
- **对话框**：健康检查宽度走 `layout::fit_width(ctx, 720)`，Narrow 下 `min(92vw,720)`；`src/main.rs` 的 `with_min_inner_size` 与 `tokens::WINDOW_MIN` 由 1120×720 改为 **720×600**，`topbar.rs` 的自绘缩放下限同步引用 `layout::MIN_*`。

证据（`docs/qa/v0.2/`，均实测，另有同内容 `-print.png`）：

| 面 | 文件（`-screen.png`） |
| --- | --- |
| ≥1280 Wide：236 侧栏 + 专辑列 + 92 播放条 | `v220-S7-wide-library-screen.png` |
| ≥1280 Wide：文件夹页（无当前曲时无右栏） | `v220-S7-wide-screen.png` |
| 840–1119 Compact：72 图标轨 + 无专辑列 + 76 播放条（1024×720） | `v220-S7-compact-screen.png` / `v220-S7-compact-library-screen.png` |
| <840 Narrow：抽屉按钮 + 两行 96 播放条（720×720） | `v220-S7-narrow-screen.png` |
| <840 Narrow：抽屉展开（遮罩 + 236 面板） | `v220-S7-drawer-screen.png` |

取舍（记录，不阻塞 D13 收口）：
- 歌词 / 智能歌单 / 均衡器三个编辑窗仍是固定 820 / 620 / 788 内容宽，未接 `dialog_width`；Narrow 下靠 egui 自动夹到屏幕内，若后续要严格 92vw，再逐窗接。
- Narrow 的运输簇右对齐（信息列固定 44%），不是稿面的绝对居中；两行结构与进度独占一行已满足。
- 抽屉开合是瞬时显示，没有 280ms 滑入：稿面要求「断点切换不播动画」，开合动画留待 S8 之后统一动效包。
- 200% DPI 四档截图待补：本轮以 1440×900 / 1024×720 / 720×720 三个逻辑尺寸取证，DPI 缩放回归放在发版前统一跑。

### S8 桌面歌词 D14（S，0.5 天）

1px 白边接受；文字阴影 `0 0 1px rgba(0,0,0,.35)`（浅壁纸）/ `rgba(255,255,255,.22)`（深壁纸），三种壁纸对比度 ≥4.5:1。

### S9 可访问性补齐（S，1–2 天）

- 顶栏动作按钮去掉 toggle 语义（讲述人不再读「… 关闭」）——需要确认这些按钮确实是开关语义（设置面板有开/关）还是纯动作，二选一并统一；
- 表格行/侧栏项补齐 `WidgetInfo` 与稳定 `Id`；
- 回归脚本改成「UIA 名称断言 + 物理点击」（已写进 `V2_REGRESSION.md` R2 条目）。

### S10 可访问性框架升级（M，3–5 天，需用户批准）

J6 与首 Tab 焦点：

- vendor `eframe` 0.31.1，把 `glow_integration.rs:264-277` 的「只对 `ViewportId::ROOT` 调 `init_accesskit`」改成遍历所有带窗口的 viewport，并在新 viewport 创建处补一次；
- 事件路由（`on_accesskit_event` → `viewport_from_window`）已就绪，不用改；
- 首 Tab 停在 `role=window` 根节点：改 accesskit 节点属性或在升级 eframe 时一并解决。

方案与风险评估见 `docs/V2.1.4_ACCESSIBILITY_PLAN.md`。**这是唯一需要动依赖的包**，也是 v2.2.0 里唯一有升级维护成本的一项。

---

## 3. v2.3.0 规模化（P2，10–14 人日）

| # | 工作 | 验收 |
| --- | --- | --- |
| C1 | 输出设备选择（WASAPI 枚举/切换） | 切设备不中断播放，切回原设备恢复 |
| C2 | 搜索索引（倒排 + 前缀） | 5 万首内输入延迟 p95 < 30ms |
| C3 | 封面懒加载与磁盘缓存 | 滚动 5 万首不卡顿，缓存目录可控 |
| C4 | 导入吞吐 | 5 万首首导 < 8 分钟（当前 28ms/首 ≈ 23 分钟） |
| C5 | CUE 多 FILE | 单 FILE 与多 FILE 两种用例都能正确切轨 |
| C6 | 大库内存 | 5 万首常驻 < 350MB |

---

## 4. 暂缓验收项（需真人或 fixture，不阻塞发版）

| 项 | 恢复条件 |
| --- | --- |
| D6 无缝播放 | 需要人耳判断切轨空隙；准备 2 张连续轨专辑 |
| D9 五态图标 | 逐态目视（播放/暂停/缓冲/错误/停止） |
| D10 SMTC 系统浮层 | 在系统媒体浮层上点按上一首/播放/下一首 |
| E3/E4 歌词逐句编辑与偏移 | 需要输入与听感配合 |
| E5 获取歌词 | 离线优先下 LRCLIB 不可达，需联网环境 |
| C10 / K5 智能歌单 | 需要先按 S5 建出构建器与 fixture |
| H1 重复歌曲 | 需要重复曲目 fixture（同曲不同码率） |
| H3 CUE | 需要单 FILE 与多 FILE 两个 CUE fixture |

---

## 5. 顺序与里程碑

1. **S3 品牌落地**（1 天）——最快见效，且不依赖其它包；
2. **S1 托盘 + S2 迷你**（2–4 天）——两个浮层一起对稿，动画 token 统一；
3. **S10 可访问性框架**（3–5 天，需批准）——唯一动依赖的包，先做可以把风险前置；
4. **S5 编辑类 + S4 曲库工具**（5–7 天）——顺带把 C10/K5/H1 的 fixture 建出来；
5. **S7 响应式骨架**（4–6 天）——单独排期，改完要重跑 DPI 四档；
6. **S6 系统面 + S8 歌词阴影 + S9 可访问性收尾**（3–4 天）——收口；
7. v2.2.0 发版，再进 v2.3.0。

**验收口径**：每个包都以「`spec2-*.png` 逐项截图比对 + `docs/qa/SURFACE_MATRIX.md` 该面状态改为『已完成』」为完成标准；截图存 `docs/qa/v0.2/v220-<包>-*.png`。

---

## 6. 风险与取舍

- **S7 响应式**是唯一可能超期的包：四档断点要重排侧栏、列表、播放条三处布局，且与 200% DPI 限制纠缠。若时间紧，可先做 1120–1279 与 840–1119 两档，<840 抽屉态延后。
- **S10 框架升级**会引入 eframe 补丁维护成本。若不批准，J6 与首 Tab 焦点继续挂账，其余包不受影响。
- **对稿不等于推翻**：现有交互逻辑已通过 90 项回归，改版只动视觉与尺寸，不动数据流；每个包改完必须重跑 `cargo test --release` 与对应组回归。
- 8 类暂缓项不会因为 v2.2.0 自动关闭——它们要的是人耳/人眼/fixture，不是代码。
