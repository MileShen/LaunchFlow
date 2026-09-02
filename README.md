# LaunchFlow 一键启动器

使用 **Tauri 2 + Rust + WebView2** 开发的现代 Windows 桌面启动器，不再使用 Python GUI。

当前版本：`2.0.0`，支持 Windows x64。

## 主要功能

- 管理多套启动场景，并一键启动场景中的多个软件；
- 从 Windows 开始菜单快捷方式中搜索、选择应用，并自动过滤卸载程序；
- 自定义添加 `.exe`、`.lnk`、`.bat`、`.cmd`、`.url`，支持启动参数和工作目录；
- 应用启用/暂停、编辑、删除和顺序调整；
- 启动器默认请求管理员权限，每个软件可单独选择管理员或普通权限启动；
- 开机自动启动工具；
- 开机自启动使用 Windows 最高权限计划任务，兼容管理员模式；
- 在场景的更多菜单中选择唯一的自启动场景，工具打开后自动运行一次；
- 浅色、深色、跟随系统三种主题；
- 兼容旧版配置，数据继续保存在 `%APPDATA%\OneClickLauncher\config.json`。

## 直接使用

双击 `dist\一键启动器.exe`。独立 EXE 不需要安装 Python；Windows 10/11 通常已自带 Microsoft Edge WebView2 Runtime。

建议把 EXE 放到固定目录后再开启开机自启动。移动 EXE 后，请关闭并重新打开一次开机自启开关。

程序默认请求管理员权限，因此 Windows 会显示 UAC 确认窗口。配置数据保存在用户目录，不会提交到 Git 仓库。

### 校验成品

发布成品的 SHA-256 校验值记录在 `dist/SHA256SUMS.txt`。PowerShell 中可以执行：

```powershell
Get-FileHash -Algorithm SHA256 .\dist\一键启动器.exe
```

## 项目结构

```text
.
├─ dist/                         # 可直接运行的 Release 成品及校验值
├─ src-tauri/                    # Rust 后端、Windows 清单和 Tauri 配置
│  ├─ capabilities/
│  ├─ icons/
│  └─ src/
├─ ui/                           # HTML、CSS、JavaScript 前端
├─ package.json                  # Tauri CLI 与构建命令
├─ package-lock.json             # Node.js 依赖锁
├─ README.md
├─ CHANGELOG.md
└─ 打包EXE.bat                    # Windows 一键构建脚本
```

## 开发与打包

环境要求：Node.js、Rust stable 和 Microsoft C++ Build Tools。

```powershell
npm.cmd install
npm.cmd run dev
npm.cmd run build
```

也可以双击 `打包EXE.bat`。Release 原始产物位于 `src-tauri\target\release\one-click-launcher.exe`，脚本会将其复制为 `dist\一键启动器.exe`。

构建缓存 `node_modules/`、`src-tauri/target/` 和 `src-tauri/gen/` 已被 Git 忽略，不应提交。

## 发布建议

- Git 仓库保存完整源码、锁文件、构建脚本和说明文档。
- `dist/一键启动器.exe` 可以随源码提交；若使用 GitHub/GitLab Release，也建议将它作为版本 `v2.0.0` 的发布附件。
- 当前项目尚未指定开源许可证。在确定许可证前，默认保留全部权利。
