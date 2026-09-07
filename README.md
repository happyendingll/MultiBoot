# MultiBoot

基于 Tauri 2 的跨平台、配置驱动托盘命令启动器。

## v1.3（1.3.0）

- 修复 Linux/Wayland 下设置窗口从托盘显示后标题栏按钮无响应的问题。
- Linux 进程内显式启用 GTK 菜单图片，避免桌面默认设置导致命令图标被过滤；不修改全局桌面设置。
- Windows/macOS 窗口和托盘行为保持不变。Ubuntu 24.04 菜单图片仍需实机验收。

## 当前进度

- Phase 1：托盘、设置窗口、基础配置、动态菜单、Shell 命令执行。
- Phase 2：新增、编辑、删除、排序、启用与禁用，并实时刷新托盘。
- Phase 3：图片上传、Base64 PNG 持久化、缩放解码及托盘菜单图标。
- Phase 4：读取并切换 Windows、Linux、macOS 系统真实自启动状态。
- Phase 5：单文件 JSON 导入导出、追加/覆盖、UUID 冲突处理和导入验证。
- Phase 6：损坏配置恢复、本地日志、Single Instance、跨平台 CI 与验证清单。

## 本地验证

```sh
npm install
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## 桌面发行包

在 macOS 上生成同时支持 Apple Silicon 与 Intel 的 Universal 2 应用：

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run build:macos-universal
```

产物位于 `src-tauri/target/universal-apple-darwin/release/bundle/macos/MultiBoot.app`。

`.github/workflows/build-desktop.yml` 可手动触发，也会在推送 `v*` 标签时运行。它在对应的原生 GitHub runner 上生成：

- macOS Universal 2：包含 `arm64` 和 `x86_64` 的 `.app.zip`
- Windows x64：NSIS `.exe` 与 MSI `.msi`
- Linux x64：AppImage 与 Debian `.deb`

未配置商业代码签名证书时，macOS 使用 ad-hoc 签名。对外发布仍建议配置 Apple Developer ID、公证及 Windows 代码签名。
