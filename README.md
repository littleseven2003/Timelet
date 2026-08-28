# Timelet（时屿）

常驻系统托盘的倒计时工具：macOS 菜单栏 / Windows 任务栏图标常驻，点击弹出面板查看倒计时与正计时条目。

## 功能

- 托盘图标常驻，左键弹出倒计时面板，右键打开配置
- 条目支持日期倒计时与正计时（已过天数）两种类型
- 数据本地持久化，重启不丢失
- 中文界面（预留多语言结构）

## 开发

环境要求：Node.js ≥ 20、pnpm ≥ 9、Rust stable。

```bash
pnpm install        # 安装前端依赖
pnpm tauri dev      # 本地开发调试
pnpm tauri build    # 构建当前平台安装包
pnpm lint           # 前端代码检查
```

## 许可证

[GPL-3.0](./LICENSE)
