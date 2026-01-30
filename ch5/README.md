# 第五章：进程

本章实现了完整的进程管理，支持进程创建、执行、等待等操作。

## 功能概述

- 进程控制块 (PCB) 管理进程资源（地址空间、上下文、PID）
- `fork` 创建子进程，复制父进程地址空间
- `exec` 根据程序名加载并执行新程序
- `wait` 等待子进程退出并回收资源
- 进程树结构维护父子关系
- 初始进程 `initproc` 作为所有用户进程的祖先

## 用户程序加载

用户程序在编译时通过 `APP_ASM` 环境变量内联到内核镜像，运行时通过 `APPS` 静态表按名称查找并加载。

## 系统调用

| 系统调用 | 功能 |
|----------|------|
| `fork` | 创建子进程（复制地址空间） |
| `exec` | 加载并执行新程序 |
| `wait` | 等待子进程退出 |
| `exit` | 退出当前进程 |
| `getpid` | 获取当前进程 PID |
| `read` | 从标准输入读取 |
| `write` | 向标准输出写入 |
| `sbrk` | 调整进程堆空间 |

## fork 的实现

`fork` 创建子进程，复制父进程的地址空间。关键在于正确复制页表和物理页面：

```rust
fn fork(&self) -> Option<Process> {
    // 复制地址空间（深拷贝所有映射的物理页）
    let address_space = self.address_space.clone_from(...);
    // 复制上下文
    let context = self.context.clone();
    // 分配新 PID
    let pid = ProcId::new();
    // ...
}
```

`fork` 返回后，父进程返回子进程 PID，子进程返回 0：

```rust
// 在子进程上下文中设置返回值为 0
*child_proc.context.context.a_mut(0) = 0;
// 父进程返回子进程 PID
pid.get_usize() as isize
```

## Exercise

见 [Exercise](./exercise.md)

## Dependencies

| 依赖 | 说明 |
|------|------|
| `xmas-elf` | ELF 文件解析 |
| `riscv` | RISC-V CSR 寄存器访问 |
| `tg-sbi` | SBI 调用封装库 |
| `tg-linker` | 链接脚本生成、内核布局定位、用户程序元数据 |
| `tg-console` | 控制台输出 (`print!`/`println!`) 和日志 |
| `tg-kernel-context` | 用户上下文及异界传送门（启用 `foreign` feature） |
| `tg-kernel-alloc` | 内核内存分配器 |
| `tg-kernel-vm` | 虚拟内存管理 |
| `tg-syscall` | 系统调用定义与分发 |
| `tg-task-manage` | 进程管理框架（启用 `proc` feature） |

## Features

| Feature | 说明 |
|---------|------|
| `nobios` | 无需外部 SBI 实现，直接从 QEMU `-bios none` 模式启动 |

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
