# MultiBoot 验证清单

自动化 CI 在 macOS、Windows、Linux 原生运行器执行前端构建、`cargo check`、测试和零警告 Clippy。

发布前仍须分别在三个真实桌面环境完成以下人工验收；CI 编译不能替代系统托盘的视觉验证。

1. 首次启动不显示设置窗口，托盘中至少有“设置”和“退出”。
2. 新增两个带完整 Shell 语法的命令，确认标题、排序、启用状态实时更新。
3. 分别上传 PNG、JPEG、WEBP、ICO，确认设置页预览和原生 Tray Menu Item 图标。
4. 执行成功及失败命令，确认 exit code、stdout、stderr 反馈。
5. 关闭设置窗口后进程继续常驻；重复启动只保留一个托盘并打开已有设置窗口。
6. 关闭/开启自启动后，分别检查 Windows Run、macOS LaunchAgent、Linux autostart 的系统真实注册状态。
7. 上传图标后导出 JSON，删除原条目，再导入 JSON，确认托盘图标恢复。
8. 导入损坏 JSON、错误版本、重复 UUID、损坏图标，确认不执行命令且应用仍可使用。

当前开发机只能实际执行 macOS 验收。Windows 与 Linux 必须在对应真实系统完成后才可签署视觉验收结论。

## 2026-09-06 开发机验收记录

- Phase 1 至 Phase 6 均在各阶段完成后通过前端构建、Rust `check`、单元测试和零警告 Clippy。
- 最终回归：`npm run build`、`cargo fmt --check`、`cargo check --locked`、`cargo test --locked`、`cargo clippy --locked --all-targets -- -D warnings` 全部通过；14 项 Rust 测试通过。
- macOS release `.app` 打包成功；LaunchServices 首次启动时设置窗口保持隐藏。
- 第二实例被 Single Instance 拦截，并唤醒已有设置窗口。
- 已在原生应用内验证 PNG 选择、预览、标准化 Base64 落盘及动态条目刷新；系统自启动状态读取为已注册。
- 发布版日志过滤为 Info，启动及重复启动只记录业务事件。
- Windows/Linux 的真实托盘图标与系统自启动仍按上方清单在对应系统验收；GitHub Actions 负责三个系统的原生编译门禁。
- macOS Universal 2 release 已在 Apple Silicon 开发机成功编译并完成启动/单实例冒烟验证；`lipo` 确认包含 `x86_64` 与 `arm64`，`codesign --verify` 通过；仍需在真实 Intel Mac 上完成 x86_64 运行时验收。
- `build-desktop.yml` 在原生 runner 生成 macOS Universal 2、Windows x64 和 Linux x64 可下载产物，避免用 macOS 交叉打包替代目标系统验证。
