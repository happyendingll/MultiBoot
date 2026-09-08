# MultiBoot

基于 Tauri 2 的跨平台、配置驱动托盘命令启动器。

## v1.3.1

- 更新应用图标，并在设置页右上角显示当前版本。
- 启动时静默检查更新，也可点击“检查更新”手动检查。
- 发现新版本后可在应用内下载安装，并在完成后自动重启。

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

`.github/workflows/build-desktop.yml` 可手动触发，也会在推送 `v*` 标签时运行。标签构建会创建对应的 GitHub Release、生成更新清单与签名，并在原生 GitHub runner 上生成：

- macOS Universal 2：包含 `arm64` 和 `x86_64` 的 `.app` 更新包
- Windows x64：NSIS `.exe` 与 MSI `.msi`
- Linux x64：AppImage 与 Debian `.deb`

自动更新依赖仓库 Secrets `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。Windows 更新优先使用 NSIS，Linux 自动更新仅支持 AppImage；`.deb` 用户需手动升级。

未配置商业代码签名证书时，macOS 使用 ad-hoc 签名。更新包本身仍由 Tauri 更新签名密钥验证；对外发布建议另外配置 Apple Developer ID、公证及 Windows 代码签名。
