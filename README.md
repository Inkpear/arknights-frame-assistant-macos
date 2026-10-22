# 明日方舟帧操小助手 (macOS)

[Arknights Frame Assistant](https://github.com/CloudTracey/arknights-frame-assistant)（简称 **AFA**）的 macOS 移植版。使用 Tauri v2 + Rust 构建，提供过帧操作、按键自定义、UI 校准等功能，支持iPad版本与应用宝模拟器版本。

## 功能特性

### 过帧与精细操作

| 功能 | 说明 |
|------|------|
| 一键技能 | 一键开启鼠标所指单位的技能 |
| 一键撤退 | 一键撤退鼠标所指单位 |
| 暂停时选中 | 在暂停时 0 帧选中鼠标所指的场上单位 |
| 暂停技能 | 在暂停时 0 帧开启鼠标所指单位的技能 |
| 暂停撤退 | 在暂停时 0 帧撤退鼠标所指单位 |
| 前进 12ms | 前进 2 倍速下的 1 逻辑帧 |
| 前进 33ms | 前进 1 倍速下的 1 逻辑帧（另一实现） |
| 前进 166ms | 前进 0.2 倍速下的 1 逻辑帧 |
| 暂停/恢复 | 模拟按下空格键暂停/恢复游戏 |
| 切换倍速 | 模拟按下 D 键切换游戏倍速 |

### UI 校准

支持对游戏界面关键位置（左暂停按钮、右暂停按钮、技能按钮、撤退按钮、倍速按钮）进行像素级精确定位，确保在任意窗口大小和分辨率下操作准确。

### 白菱形识别（兼容模式）

点击干员后，游戏会在其周围显示一个白色菱形；技能/撤退按钮位于菱形另外两条边的中点。启用兼容模式后，快速技能/快速撤退只使用视觉识别：

- 不需要额外标定，自动适应地图缩放和干员位置；
- 只截游戏窗口，不把桌面、终端或其他窗口带进检测；
- 保持一个 60fps 的游戏窗口捕获流；按快捷键后等待约 150ms 让选中干员的镜头倾斜动画结束，再取一帧识别并点击；60 秒无操作后自动停止；
- 执行快速技能/撤退时会自动隐藏设置窗口，避免透明窗口挡住游戏点击（从菜单栏可重新打开）；
- 开启后只使用白菱形识别，**不做失败回退**：未识别到菱形、没有屏幕录制权限或窗口不可用时，不会点击技能/撤退；关闭开关即可回到 UI 比例校准；
- 在「UI校准 → 白菱形识别」中切换兼容模式，可开启**试运行**（只识别、不点击）和**保存调试截图**；调试截图保存在应用配置目录的 `vision-debug/`，只保留最近 20 次。

首次使用需要在「系统设置 → 隐私与安全性 → 屏幕录制」中授权本应用；授权后通常需要重启 App。该功能需要 macOS 14+。


## 默认按键

| 功能 | 默认按键 |
|------|---------|
| 暂停/恢复 | Space |
| 切换倍速 | D |
| 一键技能 | E |
| 一键撤退 | Q |
| 暂停时选中 | W |
| 暂停技能 | S |
| 暂停撤退 | A |
| 前进 12ms | R |
| 前进 33ms | T |
| 前进 166ms | Y |

## 注意事项

- **需要辅助功能和输入监控权限**：macOS 需要在「系统设置 → 隐私与安全性 → 辅助功能 && 输入监控」中授权本应用
- **白菱形识别还需要屏幕录制权限**：在「系统设置 → 隐私与安全性 → 屏幕录制」中授权；截图只在启动快速技能/撤退或开启调试时发生，且只截游戏窗口，不会上传
- 本工具仅供学习交流使用，请勿用于商业用途

## 常见问题

### 提示"已损坏，无法打开"或"无法验证开发者"

macOS Gatekeeper 默认拦截未签名的应用。请打开终端执行以下命令移除隔离标识：

```bash
# 将 /Applications/AFA.app 替换为你实际放置应用的路径
xattr -cr /Applications/AFA.app
```

执行后重新打开应用即可。该命令仅移除了 macOS 对该文件的下载来源标记，不会影响系统安全性。

## 技术栈

- **框架**: [Tauri v2](https://v2.tauri.app/)
- **后端**: Rust — CGEvent taps 键盘监听、mado 窗口匹配、ScreenCaptureKit 游戏窗口截图、CGEvent 模拟操作
- **前端**: TypeScript + Vite + 原生 CSS
- **构建**: Bun

## 开发

```bash
# 安装依赖
bun install

# 启动开发模式
cargo tauri dev

# 构建生产版本
cargo tauri build
```

### 验证命令

```bash
# 前端类型检查 + 构建
bun x tsc --noEmit && bun x vite build

# 后端检查 + lint + 测试
cd src-tauri && cargo check && cargo clippy && cargo test

# 全量验证
bun x tsc --noEmit && bun x vite build && cd src-tauri && cargo check && cargo clippy && cargo test
```

## 鸣谢
- [CloudTracey/arknights-frame-assistant](https://github.com/CloudTracey/arknights-frame-assistant) — 本项目所有操作逻辑（过帧、暂停、技能、撤退等行为编排）的来源
- [Montag178/arknights_imgRec_AHK](https://github.com/Montag178/arknights_imgRec_AHK) — 白菱形边缘检测与按钮几何推导的参考实现
- [Tauri](https://tauri.app/) — 跨平台桌面应用框架
- [mado](https://github.com/) — macOS 窗口监控库
