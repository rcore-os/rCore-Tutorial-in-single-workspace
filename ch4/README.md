# 第四章：地址空间

本章实现了基于 RISC-V Sv39 的虚拟内存管理，为每个进程提供独立的地址空间。

## 功能概述

- Sv39 三级页表管理，内核与用户地址空间隔离
- ELF 程序加载到独立地址空间
- 异界传送门 (`MultislotPortal`) 实现跨地址空间的上下文切换
- 内核堆分配器初始化，支持动态内存分配
- 系统调用中进行用户地址翻译和权限检查

## 快速开始

在 tg-ch4 目录下执行：

```bash
cargo run                      # 基础模式
cargo run --features exercise  # 练习模式
```

> 默认会在 tg-ch4 目录下创建 tg-user 源码目录（通过 `cargo clone`）。
> 默认拉取版本为 `0.2.0-preview.1`，可通过环境变量 `TG_USER_VERSION` 覆盖。
> 若已有本地 tg-user，可通过 `TG_USER_DIR` 指定路径。

### 测试

```bash
./test.sh  # 全部测试，等价于 ./test.sh all
./test.sh base  # 基础测试
./test.sh exercise  # 练习测试
```

## 用户程序加载

tg-ch4 在构建阶段会拉取 tg-user 并编译用户程序，生成 `APP_ASM` 内联到内核镜像中，运行时解析 ELF 并映射到独立地址空间。

## 默认 QEMU 启动参数

`-machine virt -nographic -bios none`

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

## 系统调用

| 系统调用 | 功能 |
|----------|------|
| `write` | 向标准输出写入数据（需地址翻译） |
| `exit` | 退出当前进程 |
| `sched_yield` | 主动让出 CPU |
| `clock_gettime` | 获取当前时间 |
| `sbrk` | 调整进程堆空间 |

## 依赖与配置

### Features

| Feature | 说明 |
|---------|------|
| `exercise` | 练习模式测例 |

### Dependencies

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

## 练习

见 [Exercise](./exercise.md)

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
