# R2 人工复现清单（NVDA / 讲述人）

用于确认「辅助技术激活顶栏图标按钮无效」是**适配器没有转发 `Action::Click`**，还是 **egui 侧 id 映射问题**。两者修法不同，自动化侧已排除掉名称、矩形、控件类型三条，只剩这一层。

## 环境

- Windows 11，显示器缩放 100%
- RPlayer `dist\RPlayer-windows-x64\RPlayer.exe`（v2.1.3）
- 屏幕阅读器：NVDA 2024.x 或系统「讲述人」（任选其一，两者都记录更稳妥）
- 可选：AccessKit 事件日志（NVDA 的 `NVDA+F1` 只给当前对象信息，抓事件需要 [Accessibility Insights for Windows](https://accessibilityinsights.io/)，用它的 **Events → UIA** 视图）

## 步骤

1. 启动 RPlayer，键盘按 `Ctrl+,` 确认设置能打开再关掉（保证窗口有焦点、`Esc` 可用）。
2. 开启屏幕阅读器，`Tab` 往后走，**听**它念出的顶栏按钮名称：应依次出现「词」「播放队列」「均衡器」「切换到深色主题」「设置」「本地」。
   - 若名称顺序与屏幕上图标的顺序不一致 → 记下实际顺序（这是 R2 的另一种表现）。
3. 在「设置」获得焦点时，按 **`Enter`**（键盘激活）。
   - 期望：设置弹窗打开。
   - 若打开 → 说明键盘路径正常，问题只在 UIA 动作注入。
4. 用屏幕阅读器的**对象导航**（NVDA：`Insert+Numpad8` 移到对象；讲述人：`Caps+方向键`）定位到「设置」，然后按 `Insert+NumpadEnter`（NVDA 的「激活对象」）/ 讲述人 `Caps+Enter`。
   - 期望：设置弹窗打开。
   - 若只出现绿色焦点环、弹窗不开 → **复现**。
5. 同一操作对「均衡器」「切换到深色主题」各做一次，记录是否同样只出焦点环。
6. 打开 Accessibility Insights → **Events** → 勾选 `UIA`、`Invoke`、`Click`，重复第 4 步，观察是否出现任何 `UIA_AutomationPropertyChangedEventId` 或 `Action` 事件。
   - 有事件但应用没反应 → egui 侧没把 `Action::Click` 匹配到 response.id（改桥接）
   - 完全没有事件 → 适配器没转发（改 `accesskit_winit` 层或换 `egui::Button` 以外的方式）

## 记录模板

```
阅读器: NVDA 2024.4 / 讲述人
第 2 步名称顺序: 词 → 队列 → 均衡器 → 主题 → 设置 → 本地   (正常/异常：)
第 3 步 Enter 打开设置: 是 / 否
第 4 步对象激活: 打开 / 只出焦点环
第 5 步 均衡器: 打开 / 只出焦点环 ；主题: 切换 / 无变化
第 6 步 事件: 无事件 / 有事件（类型：                 ）
补充截图: tools/qa/out/screens/r2-nvda-*.png
```

## 结论怎么用

- **完全没事件** → 走「在 eframe/winit 层补 adapter 转发」，与 J6 同一个改造方向（次级窗口也没有 adapter），建议合并成一个 v2.1.4 的「可访问性适配器」专项。
- **有事件但无效** → 改 `src/app/mod.rs` 的 `update_inner`：把 `Event::AccessKitActionRequest` 里 `Action::Click` 桥接成焦点控件的 `Enter`（v2.1.2 试过一版，因无法验证而回滚，代码在 `db3fec2` 之前的提交里可查）。
