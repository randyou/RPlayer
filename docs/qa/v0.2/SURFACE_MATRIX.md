# RPlayer v2 设计面验收矩阵

基线：v2.0.17。状态：`已完成` / `部分` / `未验收`。
本矩阵只记录可核对的证据；没有截图或代码证据的面不写“已完成”。

## 视图（11）

| 设计面 | 代码 | 状态 | 证据 / 剩余 |
| --- | --- | --- | --- |
| 本地曲库 | `src/app/library_view.rs` | 已完成 | `v2-library.png`、`v2-library-1120.png` |
| 专辑 | `src/app/browse.rs` | 已完成 | `m3-albums.png` |
| 专辑详情 | `src/app/browse.rs` | 已完成 | `m3-album-detail2.png` |
| 艺术家 | `src/app/browse.rs` | 已完成 | `m3-artists.png` |
| 艺术家详情 | `src/app/browse.rs` | 已完成 | `m3-artist-detail-final.png` |
| 我的收藏 | `src/app/library_view.rs` | 已完成 | 空/有内容复用曲库表 |
| 最近播放 | `src/app/library_view.rs` | 已完成 | `m3-playlist-final.png` 同构 |
| 音乐目录 | `src/app/dialogs.rs` | 已完成 | `v2-library.png` 同顶栏 |
| 播放列表详情 | `src/app/library_view.rs` | 已完成 | `m3-playlist-final.png` |
| 智能歌单 | `src/app/smart.rs` | 已完成 | 预览卡已完成；规则行按 `DESIGN_GAP_SPEC.md` §5「不做拖拽排序」收口，删除为 24×24 命中 |
| 正在播放 | `src/app/now_playing.rs` | 已完成 | `v2-now.png`、`v2-now-dark.png` |

## 全局框架（8）

| 设计面 | 代码 | 状态 | 证据 / 剩余 |
| --- | --- | --- | --- |
| 窗口标题栏 | `src/app/topbar.rs` | 已完成 | `v2012-shell.png`；顶边只拖动不缩放（顶边缩放按设计不适用），侧/底用 `SetWindowPos` 缩放 |
| 侧边导航 | `src/app/sidebar.rs` | 已完成 | 236px、深色、分组计数 |
| 底部播放条 | `src/app/player_bar.rs` | 已完成 | 92px、48 封面、三组动作 |
| 正在播放侧栏 | `src/app/now_rail.rs` | 已完成 | 300px，≥1280 显示 |
| 播放队列 | `src/app/player_bar.rs` | 已完成 | `v2-queue.png`，360px 右抽屉 |
| 命令面板 | `src/app/command_palette.rs` | 已完成 | `v2-palette.png`，分组/键盘滚动 |
| 轻提示 | `src/app/mod.rs` | 已完成 | 底部居中 toast |
| 多选操作条 | `src/app/library_view.rs` | 已完成 | `v2017-batch-0.png` / `v2017-batch-3.png` / `v2017-batch-large.png` / `v2017-batch-crossview.png`；0 / 3 / ≥20 三态 + 跨视图“另有 N 首不在当前视图” |

## 反馈与状态（2）

| 设计面 | 代码 | 状态 | 证据 / 剩余 |
| --- | --- | --- | --- |
| 空曲库 / 空搜索 | `src/app/library_view.rs` | 已完成 | 空状态图标、说明、清除搜索 |
| 扫描进度 | `src/app/mod.rs` | 已完成 | 顶栏进度与计数 |

## 桌面覆盖层（7）

| 设计面 | 代码 | 状态 | 证据 / 剩余 |
| --- | --- | --- | --- |
| 桌面歌词 · 默认 | `src/overlay.rs` | 已完成 | `v202-lyrics-default.png` |
| 桌面歌词 · 悬停 | `src/overlay.rs` | 已完成 | `v202-lyrics-hover.png`，0.78 深色面板 |
| 桌面歌词 · 锁定 | `src/overlay.rs` | 已完成 | `v202-lyrics-locked.png`，提示走设置 |
| 桌面歌词 · 无歌词 | `src/overlay.rs` | 已完成 | `v2017-lyrics-empty.png`；空态 CTA + 切歌回归无上一首残留 |
| 迷你播放器 · 经典 | `src/overlay.rs` | 已完成 | v2.1.0 按 §3.9 对稿（`v210-mini-classic.png`）：四角 16、左上 5px 森林绿竖条、封面 92 用音符占位、标题 14.5 / 副行 9（`艺术家 · 编号`）、顶部 4×24 动作间距 8 且置顶改图钉图标、底部 28 圆播放 + 26 前后键、收藏右下；时间标签已按稿移除 |
| 迷你播放器 · 紧凑 | `src/overlay.rs` | 已完成 | v2.1.0 按 `DESIGN_GAP_SPEC.md` §7 对稿（`v210-mini-compact.png`）：无封面、左内边距 12、标题 250px 单行上限 + 悬停显示全名、副行 9、传输区居中（28 圆播放）、收藏右下；去掉稿外的「迷你播放器」角标 |
| 系统托盘菜单 | `src/app/tray.rs`、`src/platform/tray.rs` | 已完成 | v2.1.0 自绘弹窗（`v210-tray-menu-light.png` / `v210-tray-menu-dark.png`）：268×298 圆角面板、`RPLAYER` + 曲名/艺人抬头、7 行带图标与快捷键提示、两组分隔、桌面歌词选中勾、退出用 signal 色；命令走 `TrayAction::OpenMenu` + 主线程 viewport，点行即关、光标离开 360ms 自动关 |

## 右键菜单（6）

| 设计面 | 代码 | 状态 | 证据 / 剩余 |
| --- | --- | --- | --- |
| 曲目菜单 | `src/app/library_view.rs` | 已完成 | `v201-track-menu.png` |
| 专辑菜单 | `src/app/browse.rs` | 已完成 | 查看/播放/入队 |
| 艺术家菜单 | `src/app/browse.rs` | 已完成 | 查看/播放/入队 |
| 播放列表菜单 | `src/app/sidebar.rs` | 已完成 | 播放/重命名/删除 |
| 智能歌单菜单 | `src/app/sidebar.rs` | 已完成 | 播放/编辑/删除 |
| 桌面歌词菜单 | `src/overlay.rs` | 已完成 | 锁定/设置入口/关闭；滑杆仍在齿轮弹层 |

## 对话框（11）

| 设计面 | 代码 | 状态 | 证据 / 剩余 |
| --- | --- | --- | --- |
| 新建播放列表 | `src/app/dialogs.rs` | 已完成 | `v203-dialog.png`、禁用主按钮 |
| 重命名播放列表 | `src/app/dialogs.rs` | 已完成 | 同基座 |
| 删除播放列表 | `src/app/dialogs.rs` | 已完成 | 危险色主按钮 |
| 从音乐库移除 | `src/app/dialogs.rs` | 已完成 | 危险色 + 文件保留说明 |
| 编辑标签 | `src/app/dialogs.rs` | 已完成 | 字段/保存/备份说明 |
| 批量编辑标签 | `src/app/dialogs.rs` | 已完成 | 留空不改写 |
| 重复歌曲 | `src/app/dialogs.rs` | 已完成 | 分组/保留/定位 |
| 重新匹配歌词 | `src/app/dialogs.rs` | 已完成 | 候选列表/搜索 |
| 音乐文件夹 | `src/app/dialogs.rs` | 已完成 | 目录列表/扫描/检查 |
| 键盘快捷键 | `src/app/dialogs.rs` | 已完成 | `v201-shortcuts-1120.png` 21 行 |
| 关于 RPlayer | `src/app/dialogs.rs` | 已完成 | 版本/更新说明 |

## 工具窗口（4）

| 设计面 | 代码 | 状态 | 证据 / 剩余 |
| --- | --- | --- | --- |
| 均衡器 | `src/app/equalizer.rs` | 已完成 | `v203-eq.png`，深色强制 |
| 新建智能歌单 | `src/app/smart.rs` | 已完成 | `v205-smart.png`，实时预览 |
| 曲库健康检查 | `src/app/health.rs` | 已完成 | `v205-health.png`，三数据卡 |
| 编辑歌词 | `src/app/lyrics_edit.rs` | 已完成 | 标题/眉题完成；行距压缩到 6px |

## 设置（6）

| 设计面 | 代码 | 状态 | 证据 / 剩余 |
| --- | --- | --- | --- |
| 外观 | `src/app/settings.rs` | 已完成 | `v2-settings.png`，两主题/预览 |
| 桌面歌词 | `src/app/settings.rs` | 已完成 | 开关/滑杆/锁定入口文案统一 |
| 迷你播放器 | `src/app/settings.rs` | 已完成 | 样式与置顶 |
| 离线模式 | `src/app/settings.rs` | 已完成 | `v2014-offline.png`，本地可用 + 本次联网/保持离线 |
| 在线内容 | `src/app/settings.rs` | 已完成 | `v2014-online.png`，四类失败映射 + 重试 |
| 数据 | `src/app/settings.rs` | 已完成 | 备份/恢复/仅设置 |

## 汇总

- 53 个设计面；已完成 53，部分 0，未验收 0（v2.1.0 全量收口）。
- 托盘菜单与迷你播放器（经典/紧凑）已在 v2.1.0 按稿重做并收口（D1、D15 的实现部分）；D15 的像素稿本身仍待设计补，属「有实现、无正式稿」。
- 智能歌单规则行不提供拖拽排序：`DESIGN_GAP_SPEC.md` §5 明确定为「不做拖拽排序」，删除用 24×24 命中，因此该项按设计收口而非缺失。
- 顶边缩放按设计不适用：标题栏顶边只做窗口拖动，侧/底边用 `SetWindowPos` 缩放（v2.0.13 定案，v2.0.17 在矩阵内正式收口）。
- 7 个系统原生面：托盘、文件选择器、系统消息框、对话框标题栏、滚动条、IME、窗口阴影；按系统控件验收，不纳入自绘。




## 设计补充

智能歌单规则行、标题栏顶边、离线/在线设置的缺失状态已在 [`DESIGN_GAP_SPEC.md`](DESIGN_GAP_SPEC.md) 定稿，画板证据为 `design-gap-*.png`。其中标题栏顶边按设计不适用（只拖动不缩放）已在 v2.0.17 收口；智能歌单规则行排版在 v2.0.16 落地、`不做拖拽排序` 在 v2.1.0 终验中确认，该项已为 `已完成`。


