# 时屿（Timelet）图标资产

本套图标选择“时间与岛”方向，不包含字母或文字。视觉核心是一条开放的时间弧、一组极简指针与一座低矮小岛；整体强调轻盈、安静和小尺寸可识别性。

## 目录

- `AppIcon.appiconset/`：可直接放入 Xcode Assets 的 iOS、iPadOS 与 macOS App 图标集。
- `Thumbnail.imageset/`：32 / 64 / 96 px 的通用黑色缩略图集，透明背景。
- `exports/`：常用尺寸的独立 PNG，方便其他前端或桌面工程直接引用。
- `masters/`：1024 px 正式图标母版、黑色缩略图母版和原始生成图。
- `tools/export_icons.py`：从原始生成图重新导出全部工程尺寸。

## 使用建议

- iOS 使用 `timelet-app-icon-ios-1024.png` 或整个 `AppIcon.appiconset`；该版本为不含 Alpha 的 RGB PNG。
- macOS 使用 `timelet-app-icon-macos-1024.png` 或 `AppIcon.appiconset` 中的 Mac 尺寸；该版本保留透明圆角。
- 菜单栏、工具栏、通知或快捷方式优先使用 `timelet-thumbnail-black-*.png`；这些文件为纯黑图形加透明背景。
- 16–24 px 显示时建议不要再缩细线条或增加岛屿细节。

## 设计约束

- 不与字母方案混用。
- 不增加波浪、太阳、建筑、棕榈树、沙漏或日历等辅助符号。
- 保持当前留白和开放圆弧，以免图标变得笨重。
