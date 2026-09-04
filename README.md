# Timelet（时屿）

常驻系统托盘的倒计时工具：macOS 菜单栏 / Windows 任务栏图标常驻，点击弹出面板查看倒计时与正计时条目。

## 功能

- 托盘图标常驻，快捷面板最多展示 5–8 条重点日期，统一入口打开主窗口
- 条目支持日期倒计时与正计时（已过天数）两种类型
- 带时刻倒数日支持仅一次、每天和周一至周五，可持续显示距离下一次发生的小时与分钟
- 支持搜索、复制、置顶、手动排序、归档恢复及操作撤销
- 可将任意活动条目设为唯一近屿，与置顶排序独立
- 主窗口、面板与编辑器采用统一主题，支持跟随系统、浅色、深色
- 可隐藏 macOS Dock 图标或 Windows 主窗口任务栏按钮，保留菜单栏/系统托盘入口
- 数据保存在本机；覆盖前备份，读取异常停止写入，保存失败保留草稿
- 中文界面（预留多语言结构）

> 当前版本 `0.1.0`，M3 开发成果已合入主干，M4 双平台构建正在推进；尚未完成全部桌面与发布验收。独立提醒和系统通知尚未实现，重复频率只更新倒数时间。设计及当前边界见 [设计文档](docs/DESIGN.md)，测试结果见 [验收记录](docs/VALIDATION.md)。

## 开发

环境要求：Node.js ≥ 20、pnpm ≥ 9、Rust stable。

```bash
pnpm install --frozen-lockfile # 按锁文件安装前端依赖
pnpm tauri dev      # 本地开发调试
pnpm tauri build    # 构建当前平台安装包
pnpm lint           # 前端代码检查
pnpm test           # 日期与面板选择规则回归
```

## 本地打包

macOS 需 Xcode Command Line Tools；Windows 需 MSVC C++ 构建工具、Windows SDK、Rust MSVC 工具链与 WebView2。依赖需事先安装，不通过本项目脚本修改系统环境。跨平台检出后，先运行对应的 Tauri 开发或构建命令，使其同步当前平台的 Cargo 特性，再单独运行 Cargo 测试与 Clippy；直接使用另一平台留下的清单可能触发特性一致性错误。

| 执行环境 | 命令 | 产物 |
| --- | --- | --- |
| macOS | `pnpm bundle:mac` | `src-tauri/target/release/bundle/macos/Timelet.app` 与 `bundle/dmg/` 下的 DMG |
| Windows | `pnpm bundle:windows` | `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/` 下的 x64 安装程序 |

macOS 命令按本机构架构建，当前 Apple Silicon 产物不是 Intel/Universal 包。最低系统声明为 macOS 12.0，仍需对应旧系统实测。打包命令使用 Cargo 锁文件，并明确跳过分发签名；本地运行的临时签名不等于 Developer ID 签名或公证，不能承诺下载到其他设备后可直接通过系统安全检查。

Windows 安装界面为简体中文，按当前用户安装。缺少 WebView2 时需要联网下载并安装运行时；Windows ARM 上的 x64 仿真测试与原生 x64 验收分别记录。构建命令与配置已准备，不表示已经产出或验收 Windows 安装包。

公开仓库尚未发布正式安装包。Windows 构建工作流在 `feat/platform-build` 的代码或构建配置推送时触发，使用 Windows x64 构建机、Node.js 24、pnpm 11.9.0 和 Rust stable，先检查再生成 NSIS 安装包。工作流合入默认分支后可手动触发。

在仓库 Actions 中打开对应运行，检查提交号及结果后下载 `Timelet-windows-x64-<提交号>` 构建产物；内含安装程序与 `SHA256SUMS.txt`，仅保留 7 天，下载需要登录 GitHub。产物未经发布签名，不是正式 Release；构建成功不能代替安装和桌面交互验收。完整键盘操作、200% 缩放、多屏、系统变化等待办继续在验收记录中跟踪。

## 许可证

[GPL-3.0](./LICENSE)
