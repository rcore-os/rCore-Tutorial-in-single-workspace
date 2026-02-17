# tg-linker

[![Crates.io](https://img.shields.io/crates/v/tg-linker.svg)](https://crates.io/crates/tg-linker)
[![Documentation](https://docs.rs/tg-linker/badge.svg)](https://docs.rs/tg-linker)
[![License](https://img.shields.io/crates/l/tg-linker.svg)](LICENSE)

链接脚本与镜像布局支持模块，为 rCore 教学操作系统提供“链接脚本 + 启动入口 + 应用元信息”能力。

## 设计目标

- 用 Rust API 封装链接脚本细节，减少手写 ld 与符号声明成本。
- 统一内核段布局访问（如 `.text/.data/.bss` 与边界符号）。
- 提供用户应用打包后的元信息读取能力（`AppMeta`）。

## 总体架构

- `src/lib.rs`：
  - `SCRIPT` / `NOBIOS_SCRIPT`：链接脚本文本常量。
  - `KernelLayout`：定位内核段并提供 `.bss` 清零等能力。
  - `boot0!`：启动入口宏（设置栈并跳转到 Rust 主函数）。
- `src/app.rs`：
  - `AppMeta`：应用元信息定位。
  - `AppIterator`：遍历打包应用镜像。

## 主要特征

- 统一提供 BIOS 与 `nobios` 两种链接脚本。
- 提供 `KernelLayout::locate()` 定位运行时段信息。
- 提供 `AppMeta::locate().iter()` 遍历用户程序镜像。
- `no_std` 友好，适合教学内核早期启动阶段。

## 功能实现要点

- 链接脚本中定义的符号通过安全边界封装到 Rust 结构体中。
- `.bss` 清零在启动早期完成，确保全局静态变量状态正确。
- 应用元信息由 `build.rs` 生成并通过链接符号导入。

## 对外接口

- 常量：
  - `SCRIPT`
  - `NOBIOS_SCRIPT`
- 宏：
  - `boot0!`
- 结构体/迭代器：
  - `KernelLayout`
  - `KernelRegion`, `KernelRegionTitle`, `KernelRegionIterator`
  - `AppMeta`, `AppIterator`

## 使用示例

```rust
// build.rs
std::fs::write("linker.ld", tg_linker::NOBIOS_SCRIPT).unwrap();

// kernel main
unsafe { tg_linker::KernelLayout::locate().zero_bss() };
for app in tg_linker::AppMeta::locate().iter() {
    let _entry = app.as_ptr() as usize;
}
```

- 章节内真实用法：
  - `ch2/build.rs` 使用 `NOBIOS_SCRIPT` 写入链接脚本。
  - `ch2/src/main.rs` 与后续章节使用 `KernelLayout`、`AppMeta`。

## 与 ch1~ch8 的关系

- 直接依赖章节：`ch2` 到 `ch8`（含 `build-dependencies`）。
- 关键职责：组织内核/应用链接布局与运行时镜像元信息访问。
- 关键引用文件：
  - `ch2/build.rs`
  - `ch2/src/main.rs`
  - `ch8/build.rs`
  - `ch8/src/main.rs`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
