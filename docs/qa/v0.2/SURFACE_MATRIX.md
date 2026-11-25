# RPlayer v2 设计面验收矩阵

基线：v2.0.5。状态：`已完成` / `部分` / `未验收`。
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
| 智能歌单 | `src/app/smart.rs` | 部分 | 预览卡已完成；规则行未重排、无拖拽 |
| 正在播放 | `src/app/now_playing.rs` | 已完成 | `v2-now.png`、`v2-now-dark.png` |

## 全局框架（8）

| 设计面 | 代码 | 状态 | 证据 / 剩余 |
| --- | --- | --- | --- |
| 窗口标题栏 | `src/app/topbar.rs` | 部分 | 自绘按钮/拖动/侧底缩放；顶边不支持缩放 |
| 侧边导航 | `src/app/sidebar.rs` | 已完成 | 236px、深色、分组计数 |
| 底部播放条 | `src/app/player_bar.rs` | 已完成 | 92px、48 封面、三组动作 |
| 正在播放侧栏 | `src/app/now_rail.rs` | 已完成 | 300px，≥1280 显示 |
| 播放队列 | `src/app/player_bar.rs` | 已完成 | `v2-queue.png`，360px 右抽屉 |
| 命令面板 | `src/app/command_palette.rs` | 已完成 | `v2-palette.png`，分组/键盘滚动 |
| 轻提示 | `src/app/mod.rs` | 已完成 | 底部居中 toast |
| 多选操作条 | `src/app/library_view.rs` | 已完成 | 批量模式深色条 |

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
| 桌面歌词 · 无歌词 | `src/overlay.rs` | 已完成 | `v202-lyrics-empty.png` |
| 迷你播放器 · 经典 | `src/overlay.rs` | 已完成 | `v2-mini.png`，410×146 |
| 迷你播放器 · 紧凑 | `src/overlay.rs` | 已完成 | 同窗口切换，保留 |
| 系统托盘菜单 | `src/platform/tray.rs` | 已完成 | 原生 Win32 菜单，按系统样式 |

## 右键菜单（6）

| 设计面 | 代码 | 状态 | 证据 / 剩余 |
| --- | --- | --- | --- |
| 曲目菜单 | `src/app/library_view.rs` | 已完成 | `v201-track-menu.png` |
| 专辑菜单 | `src/app/browse.rs` | 已完成 | 查看/播放/入队 |
| 艺术家菜单 | `src/app/browse.rs` | 已完成 | 查看/播放/入队 |
| 播放列表菜单 | `src/app/sidebar.rs` | 已完成 | 播放/重命名/删除 |
| 智能歌单菜单 | `src/app/sidebar.rs` | 已完成 | 播放/编辑/删除 |
| 桌面歌词菜单 | `src/overlay.rs` | 部分 | 锁定/关闭可用；滑杆在齿轮弹层 |

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
| 编辑歌词 | `src/app/lyrics_edit.rs` | 部分 | 标题/眉题完成；行内时间编辑仍偏密 |

## 设置（6）

| 设计面 | 代码 | 状态 | 证据 / 剩余 |
| --- | --- | --- | --- |
| 外观 | `src/app/settings.rs` | 已完成 | `v2-settings.png`，两主题/预览 |
| 桌面歌词 | `src/app/settings.rs` | 部分 | 开关/滑杆可用；锁定入口文案待统一 |
| 迷你播放器 | `src/app/settings.rs` | 已完成 | 样式与置顶 |
| 离线模式 | `src/app/settings.rs` | 部分 | 本地优先说明；离线能力未逐项验收 |
| 在线内容 | `src/app/settings.rs` | 部分 | 封面/歌词源配置；失败态未逐项验收 |
| 数据 | `src/app/settings.rs` | 已完成 | 备份/恢复/仅设置 |

## 汇总

- 53 个设计面；已完成 47，部分 6，未验收 0。
- 部分项集中在：智能歌单规则行、顶边缩放、桌面歌词菜单、歌词编辑行、离线/在线设置。
- 7 个系统原生面：托盘、文件选择器、系统消息框、对话框标题栏、滚动条、IME、窗口阴影；按系统控件验收，不纳入自绘。
