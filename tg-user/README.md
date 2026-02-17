# tg-user

本 crate 提供 rCore Tutorial 用户态运行时与示例用户程序集合。

## 设计目标

- 提供用户态程序统一入口与基础运行时（`_start`、堆、打印等）。
- 封装用户态系统调用调用方式，减少每个示例程序重复代码。
- 为 `ch2~ch8` 提供可编译、可打包、可运行的实验用户程序集。

## 总体架构

- `cases.toml`：定义按章节打包的用户程序集合。
- `src/lib.rs`：
  - 用户态启动入口 `_start`
  - 控制台输出与常用辅助函数
  - 对 `tg-syscall(user)` 的再导出/封装
- `src/heap.rs`：用户态堆分配支持。
- `src/bin/*.rs`：各实验场景用户程序。

## 主要特征

- 提供用户态 `print!` / `println!`。
- 对常见 syscall 提供便捷封装和重导出。
- 支持按章节组织的大量测试程序。
- `no_std` 用户态运行时支持。

## 功能实现要点

- `_start` 负责用户态程序初始化并跳转到 `main`。
- 通过 `tg-syscall` 的 `user` 接口执行系统调用。
- 与章节构建脚本配合，在内核构建期将用户程序编译并打包进镜像。

## 对外接口

- 宏：
  - `print!`
  - `println!`
- 函数（示例）：
  - `getchar()`
  - `sleep(ms)`
  - `get_time()`
  - `pipe_read(...)`
  - `pipe_write(...)`
- 导出：
  - `tg_syscall::*`（用户态 syscall 接口）

## 使用示例

```rust
#[macro_use]
extern crate user_lib;

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    println!("Hello, world from user mode program!");
    0
}
```

- 章节内真实用法：
  - `ch2/build.rs` 到 `ch8/build.rs` 在构建阶段编译并打包 `tg-user` 程序。
  - `ch2/src/main.rs` 等通过 `tg_linker::AppMeta` 加载这些用户程序运行。

## 与 ch1~ch8 的关系

- 直接依赖章节：通常不是运行时 Cargo 直接依赖，而是构建/打包链路依赖（`ch2` 到 `ch8`）。
- 关键职责：提供用户态测试程序与运行时库，驱动内核 syscall/trap 路径验证。
- 关键引用文件：
  - `ch2/build.rs`
  - `ch6/build.rs`
  - `ch8/build.rs`
  - `tg-user/src/bin/*.rs`

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
