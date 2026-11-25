# QA 脚本与产物

本目录放 RPlayer 的手工/半自动 QA 工具。**所有产物写到 `out/`**（已 gitignore），不要往仓库根或 `D:\Workspace` 丢临时文件。

## 脚本

| 脚本 | 用途 | 用法 |
| --- | --- | --- |
| `capture.ps1` | 按窗口标题找窗口，同时出 `PrintWindow` 位图与屏幕区域截图 | `.\capture.ps1 -Title 'RPlayer 桌面歌词' -Out out\lyrics` |
| `shot.ps1` | 按 hwnd 截窗口（`PrintWindow(hwnd, hdc, 2)`），判断「窗口是否更新」用这个 | `.\shot.ps1 -Hwnd 1234 -Out out\main` |
| `phys-click.ps1` | 物理左键（`SetCursorPos` + `mouse_event`），egui 只认这种 | `.\phys-click.ps1 -X 500 -Y 400` |
| `post-click.ps1` | `PostMessage` 投递点击；**egui 不响应**，仅作对照实验 | `.\post-click.ps1 -Hwnd 1234 -X 300 -Y 400` |
| `click-named.ps1` | 按 AccessKit 名称查找并语义点击（需先 `agent-desktop snapshot`） | `.\click-named.ps1 -WindowId w-1234 -Name '下一首' -Count 2 -Out out\t` |
| `next-track.ps1` | 连点「下一首」，每次截图 | `.\next-track.ps1 -Count 3 -Out out\n` |
| `dump-tree.ps1` | 打印当前 AccessKit 树（role / name / ref） | `.\dump-tree.ps1` |
| `find-widgets.ps1` | 按名称关键字过滤控件 | `.\find-widgets.ps1` |
| `walk-tree.ps1` | 打印子树（调试用） | `.\walk-tree.ps1` |
| `dpi-shots.ps1` | 设 `RPLAYER_UI_SCALE` + `SetWindowPos` 固定物理尺寸后截图 | `.\dpi-shots.ps1 -Scale 1.5 -Width 1440 -Height 900 -Out out\dpi-150` |
| `scroll-bench.ps1` | 10k 曲库滚动基准，读 `RPLAYER_FRAME_LOG` 算 p50/p95 | `.\scroll-bench.ps1` |
| `perf-run.ps1` | 冷启动计时 + 内存 + 曲目数轮询 | `.\perf-run.ps1` |
| `gen-10k-wavs.ps1` | 生成 10001 个极短 WAV 到 `testdata\perf10k` | `.\gen-10k-wavs.ps1` |

## 约定

- 每次取证给产物起可读名字，最终证据再拷进 `docs/qa/v0.2/` 并写进 `SURFACE_MATRIX.md` / `REGRESSION_LOG.md`。
- 点击前用 `GetWindowRect` 现算屏幕坐标：窗口位置每次启动都会变，写死坐标必然点空。
- `agent-desktop click '@ref'` 对真 `Button` 有效，对 `chip`（role=switch）与表格行无效；那类控件只能用 `phys-click.ps1`。
- 桌面歌词覆盖层是 always-on-top 的 `NOACTIVATE` 窗口，它开着时主窗口拿不到前台，`--headed` 操作会失败；先 `toggle` 关掉覆盖层再点主窗口。
- `agent-desktop screenshot` 与 `PrintWindow` 的位图哈希相同**不代表**窗口冻结；判断是否重绘要对比内容或改用屏幕区域截图。
