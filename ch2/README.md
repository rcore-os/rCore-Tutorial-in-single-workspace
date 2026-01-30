# 第二章：批处理系统

本章实现了一个批处理操作系统，支持特权级切换和 Trap 处理，能够依次加载并运行多个用户程序。

## 功能概述

- 用户态与内核态的特权级切换
- Trap 上下文保存与恢复
- 系统调用处理 (`write`, `exit`)
- 批处理方式顺序执行用户程序

## 用户程序加载

用户程序在编译时通过 `APP_ASM` 环境变量内联到内核镜像中，运行时依次加载执行。

## 系统调用

| 系统调用 | 功能 |
|----------|------|
| `write` | 向标准输出写入数据 |
| `exit` | 退出当前程序 |

## 特权级切换与 Trap 处理

本章的核心是 `LocalContext` 结构，它保存用户态的完整寄存器状态。当用户程序发生异常或触发系统调用时：

1. 硬件自动切换到 S 态，跳转到 `stvec` 指向的 Trap 入口
2. Trap 入口保存用户寄存器到 `LocalContext`
3. 内核读取 `scause` 判断 Trap 类型，处理系统调用或异常
4. 恢复 `LocalContext` 中的寄存器，执行 `sret` 返回用户态

```rust
// 执行用户程序
unsafe { ctx.execute() };
// 从 Trap 返回后，检查原因
match scause::read().cause() {
    Trap::Exception(Exception::UserEnvCall) => handle_syscall(&mut ctx),
    // ...
}
```

## Dependencies

| 依赖 | 说明 |
|------|------|
| `riscv` | RISC-V CSR 寄存器访问 |
| `tg-sbi` | SBI 调用封装库 |
| `tg-linker` | 链接脚本生成、内核布局定位、用户程序元数据 |
| `tg-console` | 控制台输出 (`print!`/`println!`) 和日志 |
| `tg-kernel-context` | 用户上下文 `LocalContext` 及特权级切换 |
| `tg-syscall` | 系统调用定义与分发 |

## Features

| Feature | 说明 |
|---------|------|
| `nobios` | 无需外部 SBI 实现，直接从 QEMU `-bios none` 模式启动 |

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
