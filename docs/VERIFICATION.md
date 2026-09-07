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

## Linux / Wayland 标题栏回归验证

针对 Tao 0.35.3 隐藏窗口显示后标题栏无响应的问题，Linux 在设置窗口获得焦点时刷新可调整大小的窗口装饰；Windows/macOS 不编译该兼容处理。

1. 完全退出旧进程，再启动新构建；从托盘首次打开设置，不先双击标题栏或拖动窗口大小，直接验证最小化、最大化和还原。
2. 点击关闭应隐藏设置并保留托盘；从托盘重新打开，重复验证三个按钮，至少循环三次。
3. 最大化后切换到其他应用再切回，确认最大化状态和按钮正常；关闭后重新打开同样检查。
4. 通过重复启动程序唤醒设置窗口，确认按钮正常；窗口手动缩放和最小尺寸限制仍有效。
5. 在 X11（或使用 `GDK_BACKEND=x11` 启动）的环境重复检查，确认原有行为正常。

以上为待执行清单，不代表已通过真实桌面验收。

## Linux 托盘菜单图片回归验证

Linux 在创建托盘前为当前进程设置 `gtk-menu-images=true`，避免 libdbusmenu 在桌面默认关闭菜单图片时过滤命令图标；不修改系统或用户的 GTK 配置，Windows/macOS 不执行此设置。

1. 在 Ubuntu 24.04 GNOME 和 KDE 分别启动，确认带图片的命令条目显示图标，无图片条目及“设置”“退出”正常。
2. 新增图片、替换图片、删除图片、导入配置及重新排序后，确认托盘图片与设置页面一致。
3. 退出并重新启动后重复检查，确认其他应用及用户的桌面菜单图片设置未被更改。

上述跨桌面验收仍需在对应机器执行。

2026-09-08 在 Ubuntu 26.04 / KDE 上完成底层对照验证：使用与 Muda 相同的 `GtkMenuItem + GtkBox + GtkImage` 结构和同一张 PNG，通过 libdbusmenu 导出菜单。`gtk-menu-images=false` 时 `icon-data` 为 0 字节，设为 `true` 后为 95 字节。测试只修改自身进程设置，确认图片可在菜单导出阶段被过滤；尚未在 Ubuntu 24.04 实机验证。

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
