# 第四章：地址空间

本章实现了基于 RISC-V Sv39 的虚拟内存管理，为每个进程提供独立的地址空间。

## 功能概述

- Sv39 三级页表管理，内核与用户地址空间隔离
- ELF 程序加载到独立地址空间
- 异界传送门 (`MultislotPortal`) 实现跨地址空间的上下文切换
- 内核堆分配器初始化，支持动态内存分配
- 系统调用中进行用户地址翻译和权限检查

## 用户程序加载

用户程序在编译时通过 `APP_ASM` 环境变量内联到内核镜像中，运行时解析 ELF 并映射到独立地址空间。

## 系统调用

| 系统调用 | 功能 |
|----------|------|
| `write` | 向标准输出写入数据（需地址翻译） |
| `exit` | 退出当前进程 |
| `sched_yield` | 主动让出 CPU |
| `clock_gettime` | 获取当前时间 |
| `sbrk` | 调整进程堆空间 |

## 异界传送门 (MultislotPortal)

当内核与用户程序使用不同的地址空间时，上下文切换变得复杂——切换 `satp` 后代码可能无法继续执行。`MultislotPortal` 解决这个问题：

1. 传送门页面同时映射到内核和所有用户地址空间的相同虚拟地址
2. 切换时先跳转到传送门，在传送门内切换 `satp`
3. 由于传送门在两个地址空间的虚拟地址相同，切换后代码仍能执行

```rust
// 传送门位于虚拟地址空间最高页
const PROTAL_TRANSIT: VPN<Sv39> = VPN::MAX;

// 用户地址空间共享内核的传送门页表项
process.address_space.root()[portal_idx] = kernel_space.root()[portal_idx];

// 通过传送门执行用户程序
unsafe { ctx.execute(portal, ()) };
```

## Exercise

见 [Exercise](./exercise.md)

## Dependencies

| 依赖 | 说明 |
|------|------|
| `xmas-elf` | ELF 文件解析 |
| `riscv` | RISC-V CSR 寄存器访问（`satp`, `scause`） |
| `tg-sbi` | SBI 调用封装库 |
| `tg-linker` | 链接脚本生成、内核布局定位、用户程序元数据 |
| `tg-console` | 控制台输出 (`print!`/`println!`) 和日志 |
| `tg-kernel-context` | 用户上下文及异界传送门 `MultislotPortal`（启用 `foreign` feature） |
| `tg-kernel-alloc` | 内核内存分配器 |
| `tg-kernel-vm` | 虚拟内存管理 |
| `tg-syscall` | 系统调用定义与分发 |

## Features

| Feature | 说明 |
|---------|------|
| `nobios` | 无需外部 SBI 实现，直接从 QEMU `-bios none` 模式启动 |

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
