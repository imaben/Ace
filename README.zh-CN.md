# Ace

[English](README.md)

Ace 是面向 **MySQL**、**MariaDB**、**PostgreSQL** 和 **SQLite** 的跨平台
桌面数据库客户端。它是用 Rust 重写的
[Sequel Ace](https://github.com/Sequel-Ace/Sequel-Ace) 独立实现，图形界面基于
[GPUI Kit](https://crates.io/crates/gpui-kit) 0.10.0。

当前仓库版本为 **0.1.0**。项目仍处于早期阶段：已经能完成一部分工作，但许多
Sequel Ace 工作流尚未齐备。

## 项目状态

- 本仓库目前 **没有** GitHub Release、安装包或包管理器配方。试用方式是从源码构建。
- 仓库里还没有官方截图，因此本说明也不放置截图。
- `FEATURES.md` 和 `ARCHITECTURE.md` 是开发备忘，可能落后于代码。请以源码和本说明为准。

## 功能

当前代码里已经具备的能力包括：

- 默认打开桌面 GUI，同时提供查询、导出、交互式 shell 和本地 MCP 服务的命令行。
- 连接收藏夹，支持导入/导出 Sequel Ace / Sequel Pro 的 favorites plist。
- 连接方式：TCP/IP、Socket、SSH 隧道、AWS RDS IAM、Vault（Vault 的 OIDC
  浏览器回调仍是桩实现）。
- SSL/TLS（含 CA / 客户端证书），以及在非 SSL 下为 `caching_sha2_password`
  请求服务器 RSA 公钥。
- 浏览结构、内容、关系、触发器和表信息；自定义 SQL 带有高亮和补全辅助。
- 附带演示用 SQLite 数据库 `demo/demo.db`，无需服务器也能打开应用。
- 命令行可将查询结果导出为 SQL、CSV、XML、HTML、DOT 或 JSON。
- 界面字符串支持英文和简体中文。
- `ace update-check` 会查询 GitHub 上最新 Release 的标签，但 **不会** 下载或安装更新；目前也还没有可供检查的正式 Release。

## 已知限制

- 许多 Sequel Ace 功能尚未完成（多窗口会话、可视化过滤器、完整偏好设置、
  用户/进程管理、完整 Bundle 系统等）。详细清单见 `FEATURES.md`。
- 保存的密码写入本地 `credentials.json`，并不使用系统钥匙串 / 凭据管理器。
- Vault OIDC 登录尚未完整实现。
- 尚未实现 Keep-Alive、断线检测和自动重连。
- GUI 中的导入/导出覆盖范围比命令行辅助功能更窄。
- SQL 编辑器相对 Sequel Ace 仍在完善中。

## 平台支持

代码面向 Windows、macOS 和 Linux 编写，但这不等于已经在各平台提供经过完整验证的发行版。

| 平台 | 已验证内容 | 不能声称的内容 |
|------|------------|----------------|
| Windows（x64，MSVC） | 本仓库可在本地开发并编译 | 没有已发布的安装包、便携 ZIP 或商店版本 |
| macOS | GitHub Actions 在 `macos-latest` 上编译；`packaging/build-macos-app.sh` 可打包 `.app` / 可选 DMG | 没有已发布的 `.dmg`、Homebrew cask 或签名分发 |
| Linux | GitHub Actions 在安装 CI 中列出的 GPUI 系统库后于 `ubuntu-latest` 上编译 | 没有已发布的 AppImage、`.deb`、Flathub 或发行版软件包 |

CI 目前会在 `windows-latest`、`macos-latest` 和 `ubuntu-latest` 上构建并测试。CI 编译通过并不等于已经有可交付的安装包。

## 从源码构建

### 依赖

- Rust **1.100.0**（见 `rust-toolchain.toml`）。在本目录中，`rustup` 会使用该固定工具链。
- **Windows：** MSVC 构建工具以及 Windows 10 或更新的 SDK（`rc.exe` 用于嵌入应用图标）。
- **macOS：** Xcode 命令行工具。
- **Linux：** CI 中使用的开发库，包括 `pkg-config`、`libfontconfig1-dev`、
  `libfreetype-dev`、`libxkbcommon-dev`、`libxkbcommon-x11-dev`、`libxcb1-dev`、
  `libxcb-xkb-dev`、`libxcb-render0-dev`、`libxcb-shape0-dev`、`libxcb-xfixes0-dev`、
  `libx11-dev`、`libwayland-dev`、`libvulkan-dev`、`libssl-dev` 和 `cmake`。

### 命令

```bash
git clone https://github.com/imaben/Ace.git
cd Ace
cargo build -p ace-app
cargo run -p ace-app
```

`cargo run -p ace-app` 会打开图形界面。同一二进制文件也支持子命令：

```bash
# 打印版本和驱动列表
cargo run -p ace-app -- info

# 一次性查询（示例，请换成你自己的连接）
cargo run -p ace-app -- query 'sqlite://demo/demo.db' 'SELECT 1 AS ok'

# 交互式命令行
cargo run -p ace-app -- connect 'sqlite://demo/demo.db'
```

可用 `cargo build -p ace-app --release` 做发布配置构建。在 macOS 上，
`packaging/build-macos-app.sh` 会把 release 二进制打包成 `Ace.app`。
`packaging/packager.toml` 里有 Windows NSIS 和 Linux AppImage/deb 的配置草稿，但 CI 目前不会发布这些产物。

## 配置、数据与卸载

Ace 使用 `directories` crate 的 `ProjectDirs::from("com", "Ace", "Ace")` 保存设置和本地数据：

| 文件 | Windows 典型路径 | macOS 典型路径 | Linux 典型路径 |
|------|------------------|----------------|----------------|
| `settings.toml` | `%APPDATA%\Ace\Ace\settings.toml` | `~/Library/Application Support/com.Ace.Ace/settings.toml` | `~/.config/ace/settings.toml` |
| `favorites.json` | `%APPDATA%\Ace\Ace\favorites.json` | `~/Library/Application Support/com.Ace.Ace/favorites.json` | `~/.local/share/ace/favorites.json` |
| `credentials.json` | `%APPDATA%\Ace\Ace\credentials.json` | `~/Library/Application Support/com.Ace.Ace/credentials.json` | `~/.local/share/ace/credentials.json` |
| `history.sqlite3` | `%APPDATA%\Ace\Ace\history.sqlite3` | `~/Library/Application Support/com.Ace.Ace/history.sqlite3` | `~/.local/share/ace/history.sqlite3` |

如果无法解析平台目录，Ace 会回退到本地 `.ace/` 目录。

卸载源码构建时，删除 `ace` / `ace.exe` 即可（若使用过打包脚本，同时删除
`Ace.app`）。如果也要清除收藏夹、密码和查询历史，请再删除上表中的目录。
目前没有安装程序提供的卸载流程，应用本身也不会自动删除这些文件。

请将 `credentials.json` 视为敏感文件，它是本地 JSON 密码存储。

## 工作区 crate

| Crate | 作用 |
|-------|------|
| `ace-core` | 领域模型、连接信息、设置 |
| `ace-driver` | 驱动 / 连接 / 方言接口 |
| `ace-mysql` | MySQL 与 MariaDB 驱动 |
| `ace-postgres` | PostgreSQL 驱动 |
| `ace-sqlite` | SQLite 驱动 |
| `ace-sql` | SQL 拆分、分类、补全与格式化 |
| `ace-net` | SSH 隧道、AWS IAM、Vault 辅助 |
| `ace-io` | 导入 / 导出与会话格式 |
| `ace-bundle` | `.saBundle` 运行器 |
| `ace-mcp` | 本地 MCP HTTP 服务 |
| `ace-store` | 设置、收藏夹、凭据、查询历史 |
| `ace-ui` | GPUI 桌面界面 |
| `ace-app` | 应用程序二进制（`ace`） |

## 参与贡献

构建约定、测试和 Pull Request 要求见 [CONTRIBUTING.md](CONTRIBUTING.md)。
欢迎通过 GitHub Issues 提交缺陷或功能建议。

## 安全

报告漏洞前请先阅读 [SECURITY.md](SECURITY.md)。请勿在公开 Issue 中附加真实的连接机密。

## 许可证

Ace 使用 [MIT License](LICENSE)。

这是独立重写，与 Sequel Ace 没有隶属关系，也不是 Sequel Ace 源码仓库的 fork。
Sequel Ace 是采用其自身许可证的独立项目。

第三方 Rust crate 及其许可证见 [THIRD_PARTY.md](THIRD_PARTY.md)，完整清单见
[THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt)。

## 致谢

- [Sequel Ace](https://github.com/Sequel-Ace/Sequel-Ace)：产品灵感，以及 Ace 希望兼容的收藏夹 / 会话格式。
- [GPUI](https://github.com/zed-industries/zed) 与
  [GPUI Kit](https://crates.io/crates/gpui-kit)：桌面 UI 工具包。
