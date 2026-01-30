# 第八章：并发

本章实现了线程和同步原语，支持多线程编程和同步机制。

## 功能概述

- 线程创建与管理
- 互斥锁 (Mutex)
- 信号量 (Semaphore)
- 条件变量 (Condvar)
- 线程阻塞/唤醒机制

## 用户程序加载

用户程序存储在 easy-fs 磁盘镜像中，内核启动时挂载文件系统，通过文件名从文件系统加载。

## 新增或更新的系统调用

| 系统调用 | 功能 |
|----------|------|
| `thread_create` | 创建新线程 |
| `gettid` | 获取当前线程 TID |
| `waittid` | 等待线程退出 |
| `mutex_create` | 创建互斥锁 |
| `mutex_lock` | 加锁 |
| `mutex_unlock` | 解锁 |
| `semaphore_create` | 创建信号量 |
| `semaphore_up` | V 操作（释放信号量） |
| `semaphore_down` | P 操作（获取信号量） |
| `condvar_create` | 创建条件变量 |
| `condvar_signal` | 唤醒等待线程 |
| `condvar_wait` | 等待条件变量 |

## 线程

本章将进程与线程分离：`Process` 管理共享资源（地址空间、文件描述符、同步原语），`Thread` 管理执行状态（上下文、TID）：

```rust
pub struct Process {
    pub pid: ProcId,
    pub address_space: AddressSpace<Sv39, Sv39Manager>,
    pub fd_table: Vec<Option<Mutex<Fd>>>,
    pub mutex_list: Vec<Option<Arc<dyn MutexTrait>>>,
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
    // ...
}

pub struct Thread {
    pub tid: ThreadId,
    pub context: ForeignContext,
}
```

同一进程的线程共享地址空间，但各自有独立的用户栈。创建线程时在地址空间中分配新的栈区域。

## 同步与阻塞

当线程尝试获取已被占用的锁或信号量时，需要阻塞等待。

```rust
Id::SEMAPHORE_DOWN | Id::MUTEX_LOCK | Id::CONDVAR_WAIT => {
    let ctx = &mut task.context.context;
    *ctx.a_mut(0) = ret as _;
    if ret == -1 { // 获取失败，阻塞
        unsafe { (*processor).make_current_blocked() };
    } else { // 获取成功，挂起
        unsafe { (*processor).make_current_suspend() };
    }
}
```

当持有者释放锁时，将唤醒等待队列中的线程。

## 关键依赖：tg-sync

`tg-sync` 提供三种同步原语的实现，均使用 `UPIntrFreeCell` 包装以保证单处理器环境下的线程安全：

- **Semaphore（信号量）**：计数器 + 等待队列，支持 `up`/`down` 操作
  ```rust
  pub struct SemaphoreInner {
      pub count: isize,
      pub wait_queue: VecDeque<ThreadId>,
  }
  ```
  - `down(tid)`: count 减 1，若 count < 0 则将 tid 加入等待队列，返回 false
  - `up()`: count 加 1，从等待队列弹出一个线程 ID 返回

- **MutexBlocking（阻塞互斥锁）**：实现 `Mutex` trait
  ```rust
  pub struct MutexBlockingInner {
      locked: bool,
      wait_queue: VecDeque<ThreadId>,
  }
  ```
  - `lock(tid)`: 若已锁定则加入等待队列返回 false，否则获取锁返回 true
  - `unlock()`: 若等待队列非空则唤醒一个线程，否则释放锁

- **Condvar（条件变量）**：配合互斥锁使用
  ```rust
  pub struct CondvarInner {
      pub wait_queue: VecDeque<ThreadId>,
  }
  ```
  - `wait_with_mutex(tid, mutex)`: 释放锁、加入等待队列、重新尝试获取锁
  - `signal()`: 从等待队列弹出一个线程 ID

这些同步原语返回 `Option<ThreadId>`，由调度器负责实际的唤醒操作（将线程从阻塞队列移回就绪队列）。

## Exercise

见 [Exercise](./exercise.md)

## Dependencies

| 依赖 | 说明 |
|------|------|
| `virtio-drivers` | virtio 块设备驱动 |
| `xmas-elf` | ELF 文件解析 |
| `riscv` | RISC-V CSR 寄存器访问 |
| `tg-sbi` | SBI 调用封装库 |
| `tg-linker` | 链接脚本生成、内核布局定位 |
| `tg-console` | 控制台输出 (`print!`/`println!`) 和日志 |
| `tg-kernel-context` | 用户上下文及异界传送门（启用 `foreign` feature） |
| `tg-kernel-alloc` | 内核内存分配器 |
| `tg-kernel-vm` | 虚拟内存管理 |
| `tg-syscall` | 系统调用定义与分发 |
| `tg-task-manage` | 线程管理框架（启用 `thread` feature） |
| `tg-easy-fs` | 简单文件系统及管道实现 |
| `tg-signal` | 信号模块定义 |
| `tg-signal-impl` | 信号模块参考实现 |
| `tg-sync` | 同步原语（Mutex、Semaphore、Condvar）实现 |

## Features

| Feature | 说明 |
|---------|------|
| `nobios` | 无需外部 SBI 实现，直接从 QEMU `-bios none` 模式启动 |

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
