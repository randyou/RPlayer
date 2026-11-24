# RPlayer v2 · 机器可读设计规格

这四份 JSON 是设计稿的主稿。`shots/` 与 PDF 是渲染结果，用来核对，不用来猜数值。

## 读取顺序

1. `inventory.json`：先确认边界——52 个设计界面 + 7 个原生系统对话框，共 59 个可操作表面。
2. `interactions.json`：再接行为——触发、结果、键盘、aria 与 motion 引用。
3. `motion.json`：再定节奏——token、属性、曲线、减弱动态替代。
4. `flows.json`：最后串路径——播放、搜索、右键、多选、迷你、歌词、设置、托盘、扫描、备份、队列。

## 契约

- `schema` 固定为 `rplayer.design.<kind>/1`；破坏性变更才升版本。
- 每个 surface 的 `id` 稳定，例如 `VIEW.LIBRARY`、`MENU.TRACK`、`DIALOG.EDIT_TAGS`、`SETTINGS.DESKTOP_LYRICS`。
- `surfaces[].motion[]` 必须能在 `motion.json.patterns[].id` 找到；当前校验 `bad refs: 0`。
- `interactions.json` 的数组键是 `patterns`，不是 `interactions`。
- 时长 token：`instant 0` / `press 90` / `fast 140` / `base 280` / `scene 520`，单位 ms。
- 所有动效只改 `transform` 与 `opacity`；颜色变化用 token，不写十六进制。
- `prefers-reduced-motion` 或设置里的“减少动态效果”开启时，位移与缩放归零，保留颜色/透明度。
- 设置只有 6 组：外观 / 桌面歌词 / 迷你播放器 / 离线模式 / 在线内容 / 数据。快捷键与关于是独立对话框，不是设置页。
- `nativeDialogs` 只定义触发与文案，不重画系统控件。
- `nonUi` 里的模块没有窗口，不要为它们虚构界面。

## 计数

| 类别 | 数量 |
| --- | --- |
| view / 浏览 | 10 |
| global / 框架 | 8 |
| overlay / 外设 | 7 |
| menu / 右键 | 6 |
| dialog / 对话框 | 11 |
| tool / 工具窗口 | 4 |
| settings / 设置 | 6 |
| **designed** | **52** |
| native OS dialogs | 7 |
| **total** | **59** |
| motion patterns | 48 |
| interactions | 23 |
| flows | 12 |

## 实现提示

- 新增界面时先加 inventory 条目，再补 interaction 与 motion，最后才画图；不要先画图再补 ID。
- 同一条行为只允许有一个 primary action；危险动作单独置底，使用 `signal` 色。
- 每层都可用 `Esc` 退出；`Enter` 执行主操作；`↑ ↓` 在菜单和列表内移动；焦点环不可被选中底色吞掉。
- 颜色、字体、间距仍走 `tokens.css` 与 `theme::c()`；本目录不替代生产 token 实现。
