# 第二章：批处理系统

本 crate 提供一个批处理操作系统，支持特权级切换和 Trap 处理，能够依次加载并运行多个用户程序，功能与原 ch2 等价。

**差异说明**：仅支持 nobios。

## 功能概述

- 用户态与内核态的特权级切换
- Trap 上下文保存与恢复
- 系统调用处理 (`write`, `exit`)
- 批处理方式顺序执行用户程序

## 用户程序加载

tg-ch2 在构建阶段会拉取 tg-user 并编译用户程序，生成 `APP_ASM` 内联到内核镜像中，运行时依次加载执行。

## 默认 QEMU 启动参数

`-machine virt -nographic -bios none`

## 运行

请在 tg-ch2 目录下执行：

`cargo run`

默认会在 tg-ch2 目录下创建 tg-user 源码目录（通过 `cargo clone`）。
默认拉取版本为 `0.2.0-preview.1`，可通过环境变量 `TG_USER_VERSION` 覆盖。
若已有本地 tg-user，可通过 `TG_USER_DIR` 指定路径。

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
