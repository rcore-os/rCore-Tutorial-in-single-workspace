# 第八章：并发

本章在第七章"进程间通信与信号"的基础上，引入了两大核心机制：

1. **线程（Thread）**：将"进程"拆分为资源容器（Process）和执行单元（Thread），支持同一进程内的多线程并发
2. **同步原语（Synchronization Primitives）**：互斥锁（Mutex）、信号量（Semaphore）、条件变量（Condvar），解决多线程共享资源的竞争问题

通过本章的学习和实践，你将理解：

- 线程与进程的区别和联系
- 线程的创建、退出和等待机制
- 竞态条件（Race Condition）和临界区（Critical Section）的概念
- 互斥锁的原理和实现（自旋锁 vs 阻塞锁）
- 信号量的 P/V 操作及其应用（互斥和同步）
- 条件变量与管程（Monitor）的概念
- 死锁的概念和检测算法

> **前置知识**：建议先完成第一章至第七章的学习，理解裸机启动、Trap 处理、系统调用、多任务调度、虚拟内存、进程管理、文件系统、管道和信号。

## 项目结构

```
ch8/
├── .cargo/
│   └── config.toml     # Cargo 配置：交叉编译目标和 QEMU runner
├── .gitignore           # Git 忽略规则
├── build.rs            # 构建脚本：编译用户程序，打包 easy-fs 磁盘镜像
├── Cargo.toml          # 项目配置与依赖
├── LICENSE             # GPL v3 许可证
├── README.md           # 本文档
├── rust-toolchain.toml # Rust 工具链配置
├── test.sh             # 自动测试脚本
└── src/
    ├── main.rs         # 内核主体：初始化、调度循环、系统调用实现（含线程和同步原语）
    ├── fs.rs           # 文件系统管理 + 统一的 Fd 枚举
    ├── process.rs      # 进程与线程结构：Process（资源容器）+ Thread（执行单元）
    ├── processor.rs    # 处理器管理：PThreadManager（双层管理器）
    └── virtio_block.rs # VirtIO 块设备驱动
```

<a id="source-nav"></a>

## 源码阅读导航索引

[返回根文档导航总表](../README.md#chapters-source-nav-map)

本章建议按“进程/线程拆分 -> 双层管理器 -> 同步阻塞/唤醒”主线阅读。

| 阅读顺序 | 文件 | 重点问题 |
|---|---|---|
| 1 | `src/process.rs` | `Process` 与 `Thread` 如何分离资源与执行职责？ |
| 2 | `src/processor.rs` | `PThreadManager` 如何同时管理 PID 与 TID 两层关系？ |
| 3 | `src/main.rs` 初始化路径 | `init_thread`、`init_sync_mutex` 如何把线程与同步 syscall 接入系统？ |
| 4 | `src/main.rs` Trap 主循环 | `SEMAPHORE_DOWN/MUTEX_LOCK/CONDVAR_WAIT` 返回 `-1` 时为何转为阻塞态？ |
| 5 | `src/main.rs` 的 `impls::Thread/SyncMutex` | 线程创建、join、锁/信号量/条件变量的唤醒路径如何闭环？ |

配套建议：结合 `tg-sync` 与 `tg-task-manage(thread feature)` 注释阅读，重点把“阻塞队列 -> re_enque -> 再调度”串起来。

## DoD 验收标准（本章完成判据）

- [ ] 能清楚区分 `Process`（资源容器）与 `Thread`（执行单元）的职责边界
- [ ] 能说明 `PThreadManager` 如何维护 PID/TID 双层关系
- [ ] 能解释 `thread_create/gettid/waittid` 的核心语义与返回值
- [ ] 能解释阻塞型同步原语中“阻塞 -> 唤醒 -> 重新入队”的完整路径
- [ ] 能执行 `./test.sh base`（练习时补充 `./test.sh exercise`）

## 概念-源码-测试三联表

| 核心概念 | 源码入口 | 自测方式（命令/现象） |
|---|---|---|
| 进程线程解耦 | `ch8/src/process.rs` | 能描述共享资源和独立上下文分别放在哪一层 |
| 双层调度管理 | `ch8/src/processor.rs` | 能解释为何调度粒度从进程变为线程 |
| 线程系统调用 | `ch8/src/main.rs` 的 `impls::Thread` | 运行线程测试程序可看到创建与等待成功 |
| 同步阻塞/唤醒 | `ch8/src/main.rs` 的 `impls::SyncMutex` + Trap 分支 | 锁/信号量/条件变量场景可重现阻塞与唤醒 |

遇到构建/运行异常可先查看根文档的“高频错误速查表”。

## 一、环境准备

### 1.1 安装 Rust 工具链

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

验证：

```bash
rustc --version    # 要求 >= 1.85.0（支持 edition 2024）
cargo --version
```

### 1.2 添加 RISC-V 64 编译目标

```bash
rustup target add riscv64gc-unknown-none-elf
```

### 1.3 安装 QEMU 模拟器

**Ubuntu / Debian：**

```bash
sudo apt update
sudo apt install qemu-system-misc
```

**macOS（Homebrew）：**

```bash
brew install qemu
```

验证：

```bash
qemu-system-riscv64 --version    # 建议 >= 7.0
```

### 1.4 安装额外工具

```bash
cargo install cargo-clone
cargo install cargo-binutils
rustup component add llvm-tools
```

### 1.5 获取源代码

**方式一：只获取本实验**

```bash
cargo clone tg-ch8
cd tg-ch8
```

**方式二：获取所有实验**

```bash
git clone https://github.com/rcore-os/rCore-Tutorial-in-single-workspace.git
cd rCore-Tutorial-in-single-workspace/ch8
```

## 二、编译与运行

### 2.1 编译

```bash
cargo build
```

构建过程与第六、七章相同：`build.rs` 会自动下载编译 `tg-user` 用户程序，打包到 `fs.img` 磁盘镜像中。

> 环境变量说明：
> - `TG_USER_DIR`：指定本地 tg-user 源码路径
> - `TG_USER_VERSION`：指定 tg-user 版本（默认 `0.2.0-preview.1`）
> - `TG_SKIP_USER_APPS`：跳过用户程序编译
> - `LOG`：设置日志级别

### 2.2 运行（基础模式）

```bash
cargo run
```

QEMU 命令（与第六、七章相同，挂载 fs.img 块设备）：

```bash
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -drive file=target/riscv64gc-unknown-none-elf/debug/fs.img,if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
    -kernel target/riscv64gc-unknown-none-elf/debug/tg-ch8
```

### 2.3 运行（练习模式）

```bash
cargo run --features exercise
```

练习模式加载不同的用户测例集（`ch8_exercise`），用于测试死锁检测等扩展功能。

### 2.4 预期输出

```
[tg-ch8 ...] Hello, world!
[ INFO] .text    ---> 0x80200000..0x8023xxxx
[ INFO] .rodata  ---> 0x8023xxxx..0x8024xxxx
[ INFO] .data    ---> 0x8024xxxx..0x81exxxxx
[ INFO] .boot    ---> 0x81exxxxx..0x81exxxxx
[ INFO] (heap)   ---> 0x81exxxxx..0x83200000
[ INFO] MMIO range -> 0x10001000..0x10002000

Rust user shell
>> ch8b_threads
...
threads test passed!
Shell: Process 2 exited with code 0
>>
```

你可以在 Shell 中运行各种多线程和同步测试程序：
- `ch8b_threads`：基础多线程创建和 join 测试
- `ch8b_sync_sem`：使用信号量的同步测试
- `ch8b_sync_condvar`：使用条件变量的同步测试

### 2.5 运行测试

```bash
./test.sh           # 运行全部测试（base + exercise）
./test.sh base      # 运行基础测试
./test.sh exercise  # 运行练习测试
./test.sh all       # 等价于 ./test.sh
```

---

## 三、操作系统核心概念

### 3.1 为什么需要线程？

在前几章中，**进程** 既是资源容器也是执行单元。这种模型在以下场景效率不高：

| 场景 | 问题 |
|------|------|
| 并行计算 | 一个大型计算任务需要多个 CPU 核心同时执行 |
| 交替等待 I/O | 进程中的多个任务可能分别等待不同的 I/O 完成 |
| 服务并发 | 服务器需要同时处理多个客户端请求 |

如果每个并发任务都创建一个独立进程，会带来巨大的开销：
- **地址空间复制**：fork 需要复制整个地址空间
- **资源隔离**：进程之间不共享内存，通信需要通过管道等机制
- **切换开销**：进程切换需要切换页表（刷新 TLB）

**线程** 解决了这些问题：同一进程的多个线程**共享地址空间**和文件描述符，但各自有独立的**执行上下文**（寄存器、栈）。线程间切换不需要切换页表，通信可以直接通过共享内存。

```text
第七章：一个进程 = 一个线程
┌────────────────────────────┐
│         Process             │
│  地址空间 + 文件 + 上下文    │
└────────────────────────────┘

第八章：一个进程 = 多个线程
┌────────────────────────────┐
│         Process             │
│  地址空间 + 文件 + 同步原语  │
│  ┌────────┐ ┌────────┐     │
│  │Thread 0│ │Thread 1│ ... │
│  │上下文   │ │上下文   │     │
│  │用户栈   │ │用户栈   │     │
│  └────────┘ └────────┘     │
└────────────────────────────┘
```

### 3.2 线程模型

#### 线程与进程的关系

| 属性 | 进程（Process） | 线程（Thread） |
|------|----------------|----------------|
| 地址空间 | 独享 | 共享（同进程线程共享） |
| 文件描述符 | 独享 | 共享 |
| 同步原语 | 独享 | 共享 |
| 执行上下文 | — | 独享（寄存器、栈指针） |
| TID | — | 独享 |
| 调度 | — | 线程是调度的基本单位 |

#### 内核数据结构

```rust
/// 线程（执行单元）
pub struct Thread {
    pub tid: ThreadId,         // 线程 ID
    pub context: ForeignContext, // 执行上下文 (LocalContext + satp)
}

/// 进程（资源容器）
pub struct Process {
    pub pid: ProcId,
    pub address_space: AddressSpace<Sv39, Sv39Manager>,
    pub fd_table: Vec<Option<Mutex<Fd>>>,
    pub signal: Box<dyn Signal>,
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,  // 本章新增
    pub mutex_list: Vec<Option<Arc<dyn MutexTrait>>>, // 本章新增
    pub condvar_list: Vec<Option<Arc<Condvar>>>,      // 本章新增
}
```

#### 线程相关系统调用

| 系统调用 | 功能 |
|----------|------|
| `thread_create(entry, arg)` | 在当前进程中创建新线程，入口为 entry，参数为 arg |
| `gettid()` | 获取当前线程的 TID |
| `waittid(tid)` | 等待指定线程退出，返回其退出码 |

**thread_create 的关键步骤：**

```text
1. 在地址空间中搜索未映射的页面区域
2. 分配 2 页用户栈
3. 创建新的 LocalContext，设置入口地址和参数
4. 创建 Thread 对象，加入线程管理器
5. 返回新线程的 TID
```

#### 管理器的变化

| 特性 | 第七章 | 第八章 |
|------|--------|--------|
| 全局管理器 | `PManager` | `PThreadManager` |
| 管理层次 | 单层（进程） | 双层（进程 + 线程） |
| 调度单位 | 进程 | 线程 |
| task-manage feature | `proc` | `thread` |

### 3.3 竞态条件与互斥

#### 竞态条件（Race Condition）

当多个线程同时访问**共享资源**且至少一个是写操作时，就可能出现竞态条件：

```text
线程 A：                   线程 B：
  load count → 5             load count → 5
  add 1 → 6                  add 1 → 6
  store count ← 6            store count ← 6

期望结果：count = 7
实际结果：count = 6  ← 数据竞争！
```

#### 临界区与互斥

解决竞态条件的方法是**互斥**（Mutual Exclusion）：确保同一时刻只有一个线程可以进入**临界区**（访问共享资源的代码段）。

关键术语：

| 概念 | 说明 |
|------|------|
| **临界区（Critical Section）** | 访问共享资源的代码段 |
| **互斥（Mutual Exclusion）** | 同一时刻只有一个线程在临界区 |
| **原子性（Atomicity）** | 操作不可被中断 |
| **死锁（Deadlock）** | 多个线程互相等待对方释放资源 |
| **饥饿（Starvation）** | 某个线程长期无法获取资源 |

### 3.4 互斥锁（Mutex）

#### 基本概念

互斥锁是最基本的同步原语，提供 `lock` 和 `unlock` 两个操作：

```text
lock(mutex)           ← 获取锁（如果锁被占用则阻塞）
  // 临界区操作
unlock(mutex)         ← 释放锁
```

#### 锁的类型

| 类型 | 获取失败时 | 优点 | 缺点 |
|------|-----------|------|------|
| **自旋锁（Spin Lock）** | 忙等待（循环检查） | 无上下文切换开销 | 浪费 CPU 时间 |
| **阻塞锁（Blocking Lock）** | 阻塞线程，进入等待队列 | 不浪费 CPU | 有上下文切换开销 |

本实现中使用**阻塞锁**（`MutexBlocking`）：

```rust
pub struct MutexBlockingInner {
    locked: bool,                    // 是否已锁定
    wait_queue: VecDeque<ThreadId>,  // 等待队列
}
```

- `lock(tid)`：若已锁定，将 tid 加入等待队列并返回 false（阻塞）；否则获取锁返回 true
- `unlock()`：若等待队列非空，弹出一个线程 ID 返回（唤醒）；否则释放锁

#### 锁的性质

一个好的锁实现应满足三个性质：
1. **互斥性**（Mutual Exclusion）：同时只有一个线程持有锁
2. **公平性**（Fairness）：每个等待的线程最终都能获取锁
3. **性能**（Performance）：获取和释放锁的开销尽可能小

#### 系统调用

| 系统调用 | 功能 |
|----------|------|
| `mutex_create(blocking)` | 创建互斥锁（blocking=true 为阻塞锁） |
| `mutex_lock(mutex_id)` | 加锁 |
| `mutex_unlock(mutex_id)` | 解锁 |

### 3.5 信号量（Semaphore）

#### 基本概念

信号量由 Dijkstra 在 1965 年提出，是一种更通用的同步原语。信号量是一个带有等待队列的**计数器**，提供两个原子操作：

| 操作 | 荷兰语名 | 语义 |
|------|----------|------|
| **P（down/wait）** | Proberen（尝试） | 计数器减 1，若 < 0 则阻塞 |
| **V（up/signal）** | Verhogen（增加） | 计数器加 1，若有等待者则唤醒一个 |

```rust
pub struct SemaphoreInner {
    pub count: isize,                    // 计数器
    pub wait_queue: VecDeque<ThreadId>,  // 等待队列
}
```

#### 信号量的用途

| 初始值 | 用途 | 说明 |
|--------|------|------|
| 1 | **互斥**（二值信号量） | 等价于互斥锁 |
| 0 | **同步** | 一个线程等待另一个线程的事件 |
| N | **资源计数** | 控制最多 N 个线程同时访问资源池 |

**示例：使用信号量实现互斥**

```text
Semaphore sem = new Semaphore(1);  // 初始值 = 1

Thread A:            Thread B:
  P(sem)               P(sem)      ← 如果 A 先执行，B 阻塞
  // 临界区             // 临界区
  V(sem)               V(sem)
```

**示例：使用信号量实现同步**

```text
Semaphore done = new Semaphore(0);  // 初始值 = 0

Thread A（生产者）:      Thread B（消费者）:
  produce_data();          P(done);     ← 阻塞等待 A 完成
  V(done);                 consume_data();
```

#### 系统调用

| 系统调用 | 功能 |
|----------|------|
| `semaphore_create(res_count)` | 创建信号量（初始计数 = res_count） |
| `semaphore_up(sem_id)` | V 操作（释放资源，可能唤醒等待者） |
| `semaphore_down(sem_id)` | P 操作（获取资源，可能阻塞） |

### 3.6 条件变量（Condvar）

#### 为什么需要条件变量？

互斥锁只能保证"互斥"，但不能高效地实现"等待某个条件成立"。例如，生产者-消费者问题中，消费者需要等待"缓冲区非空"这个条件：

```text
// 用互斥锁的低效实现（忙等待）
loop {
    mutex_lock(m);
    if buffer.is_empty() {
        mutex_unlock(m);
        yield();         // 释放锁后让出 CPU，再重新尝试
        continue;
    }
    data = buffer.pop();
    mutex_unlock(m);
    break;
}
```

条件变量提供了更高效的解决方案：

```text
mutex_lock(m);
while buffer.is_empty() {
    condvar_wait(cv, m);   // 原子地释放锁 + 阻塞 + 被唤醒后重新获取锁
}
data = buffer.pop();
mutex_unlock(m);
```

#### 管程（Monitor）

条件变量通常和互斥锁配合使用，二者合在一起被称为**管程**。管程有三种语义：

| 语义 | 特点 |
|------|------|
| **Hoare 语义** | signal 后立即切换到被唤醒线程 |
| **Hansen 语义** | signal 必须是临界区最后一个操作 |
| **Mesa 语义** | signal 只是"提示"，被唤醒线程需重新检查条件 |

本实现采用类似 **Mesa 语义**：`condvar_signal` 只是将等待线程加入就绪队列，被唤醒线程重新尝试获取锁时可能发现条件已不满足，因此需要在 `while` 循环中使用 `condvar_wait`。

#### 内部实现

```rust
pub struct CondvarInner {
    pub wait_queue: VecDeque<ThreadId>,
}
```

- `wait_with_mutex(tid, mutex)`：
  1. 释放 mutex（可能唤醒另一个等待 mutex 的线程）
  2. 将 tid 加入条件变量的等待队列
  3. 返回 false 表示阻塞
- `signal()`：从等待队列弹出一个线程 ID 返回

#### 系统调用

| 系统调用 | 功能 |
|----------|------|
| `condvar_create(arg)` | 创建条件变量 |
| `condvar_signal(condvar_id)` | 唤醒一个等待线程 |
| `condvar_wait(condvar_id, mutex_id)` | 等待条件变量（释放锁 + 阻塞 + 重获取锁） |

### 3.7 线程阻塞与唤醒

当线程尝试获取已被占用的同步原语时，需要进入**阻塞态**：

```text
线程 A：mutex_lock(0)
  → MutexBlocking::lock(tid_A) 返回 false
  → 系统调用返回 ret = -1
  → 主循环判断 Id::MUTEX_LOCK && ret == -1
  → processor.make_current_blocked()
  → 线程 A 从就绪队列移除

线程 B：mutex_unlock(0)
  → MutexBlocking::unlock() 返回 Some(tid_A)
  → processor.re_enque(tid_A)
  → 线程 A 重新加入就绪队列
```

关键代码（在主循环中）：

```rust
Id::SEMAPHORE_DOWN | Id::MUTEX_LOCK | Id::CONDVAR_WAIT => {
    *ctx.a_mut(0) = ret as _;
    if ret == -1 {
        // 资源不可用，阻塞当前线程
        processor.make_current_blocked();
    } else {
        // 成功获取，正常挂起（时间片轮转）
        processor.make_current_suspend();
    }
}
```

### 3.8 死锁

#### 死锁的定义

当一组线程中的每个线程都在等待另一个线程持有的资源时，就发生了**死锁**（Deadlock）。

经典示例——哲学家就餐问题：

```text
哲学家 A：持有叉子 1，等待叉子 2
哲学家 B：持有叉子 2，等待叉子 3
哲学家 C：持有叉子 3，等待叉子 1
→ 循环等待 → 死锁！
```

#### 死锁的四个必要条件

| 条件 | 说明 |
|------|------|
| **互斥** | 资源同时只能被一个线程使用 |
| **持有并等待** | 线程持有资源的同时等待其他资源 |
| **非抢占** | 资源只能由持有者主动释放 |
| **循环等待** | 存在线程间的资源等待环 |

### 3.9 系统调用汇总

| syscall ID | 名称 | 功能 | 状态 |
|-----------|------|------|------|
| 1000 | `thread_create` | 创建线程 | **新增** |
| 1001 | `gettid` | 获取线程 TID | **新增** |
| 1002 | `waittid` | 等待线程退出 | **新增** |
| 1010 | `mutex_create` | 创建互斥锁 | **新增** |
| 1011 | `mutex_lock` | 加锁 | **新增** |
| 1012 | `mutex_unlock` | 解锁 | **新增** |
| 1020 | `semaphore_create` | 创建信号量 | **新增** |
| 1021 | `semaphore_up` | V 操作 | **新增** |
| 1022 | `semaphore_down` | P 操作 | **新增** |
| 1030 | `condvar_create` | 创建条件变量 | **新增** |
| 1031 | `condvar_signal` | 唤醒等待线程 | **新增** |
| 1032 | `condvar_wait` | 等待条件变量 | **新增** |
| 469 | `enable_deadlock_detect` | 启用/禁用死锁检测（练习） | 练习 |
| 59 | `pipe` | 创建管道 | 继承 |
| 129 | `kill` | 发送信号 | 继承 |
| 56/57 | `open`/`close` | 打开/关闭文件 | 继承 |
| 63/64 | `read`/`write` | 读取/写入 | 继承 |
| 93 | `exit` | 退出 | 继承 |
| 220/221 | `fork`/`exec` | 创建/替换进程 | 继承 |

---

## 四、代码解读

### 4.1 `src/main.rs` —— 内核主体

**与第七章的区别：**
- 新增 `tg_syscall::init_thread()` 初始化线程系统调用
- 新增 `tg_syscall::init_sync_mutex()` 初始化同步原语系统调用
- 全局管理器从 `PManager` 变为 `PThreadManager`（双层管理）
- 初始化时创建的是 `(Process, Thread)` 对
- 主调度循环中新增**线程阻塞处理**：
  - `SEMAPHORE_DOWN`/`MUTEX_LOCK`/`CONDVAR_WAIT` 返回 -1 时，调用 `make_current_blocked()` 将线程移出就绪队列
  - 返回非 -1 时，正常挂起（时间片轮转）
- `impls` 模块新增 `Thread` trait（thread_create/gettid/waittid）和 `SyncMutex` trait

### 4.2 `src/process.rs` —— 进程与线程

**核心变化：Process + Thread 分离**

| 结构 | 管理内容 |
|------|----------|
| `Thread` | TID、ForeignContext（寄存器 + satp） |
| `Process` | PID、地址空间、fd_table、signal、**semaphore_list**、**mutex_list**、**condvar_list** |

- `from_elf()`：同时创建 Process 和 Thread
- `fork()`：深拷贝地址空间和 fd_table，**同步原语列表不继承**（子进程创建空列表）
- `exec()`：替换地址空间和主线程上下文

### 4.3 `src/processor.rs` —— 双层管理器

```rust
pub type ProcessorInner = PThreadManager<Process, Thread, ThreadManager, ProcManager>;
```

- `ThreadManager`：维护所有 Thread 实体和就绪队列（FIFO 调度）
- `ProcManager`：维护所有 Process 实体
- `find_next()`：从就绪队列取出下一个 Thread 执行
- `make_current_blocked()`：将当前 Thread 标记为阻塞态
- `re_enque(tid)`：将被唤醒的 Thread 重新加入就绪队列

### 4.4 `src/fs.rs` —— 文件系统（与第七章相同）

统一的 `Fd` 枚举（File / PipeRead / PipeWrite / Empty），所有线程共享同一个 `fd_table`。

### 4.5 `Cargo.toml` —— 依赖说明

与第七章相比新增的依赖：

| 依赖 | 说明 |
|------|------|
| `tg-sync` | 同步原语实现（MutexBlocking、Semaphore、Condvar） |
| `tg-task-manage`（`thread` feature） | 双层管理器框架（PThreadManager） |

---

## 五、编程作业：死锁检测

### 5.1 问题描述

目前的 mutex 和 semaphore 相关的系统调用不会分析资源的依赖情况，用户程序可能出现死锁。我们希望系统中加入死锁检测机制，当发现可能发生死锁时拒绝对应的资源获取请求。

### 5.2 银行家算法

一种检测死锁的算法如下：

定义三个数据结构：

- **可利用资源向量 Available**：含有 m 个元素的一维数组，每个元素代表可利用的某一类资源的数目，其初值是该类资源的全部可用数目。`Available[j] = k` 表示第 j 类资源的可用数量为 k。
- **分配矩阵 Allocation**：n x m 矩阵，表示每类资源已分配给每个线程的资源数。`Allocation[i,j] = g` 表示线程 i 当前已分得第 j 类资源的数量为 g。
- **需求矩阵 Need**：n x m 矩阵，表示每个线程还需要的各类资源数量。`Need[i,j] = d` 表示线程 i 还需要第 j 类资源的数量为 d。

算法运行过程：

1. 设置两个向量：
   - 工作向量 `Work`，初始时 `Work = Available`
   - 结束向量 `Finish[0..n-1] = false`

2. 从线程集合中找到一个能满足下述条件的线程：
   ```
   Finish[i] == false
   Need[i,j] <= Work[j]
   ```
   若找到，执行步骤 3；否则执行步骤 4。

3. 当线程 `thr[i]` 获得资源后，可顺利执行直至完成，释放分配给它的资源：
   ```
   Work[j] = Work[j] + Allocation[i,j]
   Finish[i] = true
   ```
   跳转回步骤 2。

4. 如果 `Finish[0..n-1]` 都为 true，则系统处于安全状态；否则系统处于不安全状态，即出现死锁。

### 5.3 新增系统调用

**enable_deadlock_detect**：

- syscall ID: 469
- 功能：为当前进程启用或禁用死锁检测功能

```rust
fn enable_deadlock_detect(&self, _caller: Caller, is_enable: i32) -> isize
```

- 参数：`is_enable` 为 1 表示启用死锁检测，0 表示禁用
- 说明：
  - 开启后，`mutex_lock` 和 `semaphore_down` 如果检测到死锁，应拒绝并返回 `-0xDEAD`
  - 简便起见可对 mutex 和 semaphore 分别进行检测
- 返回值：成功返回 0，出错返回 -1

### 5.4 实验要求

目录结构：

```
tg-ch8/
├── Cargo.toml（内核配置文件）
├── src/（内核源代码，需要修改）
│   ├── main.rs（内核主函数，包括系统调用接口实现）
│   ├── fs.rs（文件系统相关）
│   ├── process.rs（进程结构）
│   ├── processor.rs（进程/线程管理器）
│   └── virtio_block.rs（VirtIO 块设备实现）
└── tg-user/（用户程序，运行时自动拉取，无需修改）
    └── src/bin（测试用例）
```

> **说明**：
> - `tg-user` 会在运行时自动拉取到 `tg-ch8/tg-user` 目录下
> - 只需修改 `tg-ch8/src/` 目录下的内核代码

运行练习测例：

```bash
cargo run --features exercise
```

然后在终端中输入 `ch8_usertest` 运行，这个测例打包了所有你需要通过的测例。

运行自动化测试：

```bash
./test.sh exercise
```

> **说明**：本次实验框架变动较大，不要求合并之前的实验内容，只需通过 ch8 的全部测例和其他章节的基础测例即可。

---

## 六、本章小结

通过本章的学习和实践，你完成了操作系统中并发编程的核心机制：

1. **线程模型**：将进程拆分为资源容器（Process）和执行单元（Thread），支持同一进程内的多线程并发
2. **互斥锁**：通过 lock/unlock 保证临界区互斥访问，使用等待队列实现阻塞式互斥
3. **信号量**：P/V 操作支持计数型资源管理，可用于互斥和同步
4. **条件变量**：配合互斥锁使用，高效实现"等待条件成立"的模式
5. **线程阻塞与唤醒**：资源不可用时阻塞，资源释放时唤醒等待者
6. **死锁**：理解死锁的四个必要条件和银行家算法

## 七、思考题

1. **线程 vs 进程**：在什么场景下应该使用多线程而非多进程？反过来呢？请从安全性、性能和编程难度三个角度比较。

2. **自旋锁 vs 阻塞锁**：本实现使用了阻塞锁（`MutexBlocking`），为什么不用自旋锁？在什么场景下自旋锁比阻塞锁更合适？

3. **信号量 vs 条件变量**：两者都能实现线程同步。在"生产者-消费者"问题中，使用信号量和条件变量各有什么优劣？

4. **死锁检测 vs 死锁预防**：银行家算法是一种死锁检测/预防方法。你还知道哪些方法？各有什么优缺点？

5. **fork 与线程**：在多线程进程中调用 fork 会发生什么？Linux 中有什么特殊处理？本实现中是如何处理的？

6. **公平性**：本实现中的就绪队列使用 FIFO 调度。如果一个线程频繁获取和释放锁，会不会导致其他线程饥饿？如何改进？

7. **条件变量的 while 循环**：为什么 `condvar_wait` 通常需要放在 `while` 循环中而不是 `if` 语句中？请用 Mesa 语义解释。

## 参考资料

- [rCore-Tutorial-Guide 第八章](https://learningos.github.io/rCore-Tutorial-Guide/)
- [rCore-Tutorial-Book 第八章](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter8/index.html)
- [OSTEP: Concurrency](https://pages.cs.wisc.edu/~remzi/OSTEP/)
- [Dijkstra's Semaphore (1965)](https://en.wikipedia.org/wiki/Semaphore_(programming))
- [Dining Philosophers Problem](https://en.wikipedia.org/wiki/Dining_philosophers_problem)

---

## 附录：rCore-Tutorial 组件分析表

### 表 1：tg-ch1 ~ tg-ch8 操作系统内核总体情况描述表

| 操作系统内核 | 所涉及核心知识点 | 主要完成功能 | 所依赖的组件 |
|:-----|:------------|:---------|:---------------|
| **tg-ch1** | 应用程序执行环境<br>裸机编程（Bare-metal）<br>SBI（Supervisor Binary Interface）<br>RISC-V 特权级（M/S-mode）<br>链接脚本（Linker Script）<br>内存布局（Memory Layout）<br>Panic 处理 | 最小 S-mode 裸机程序<br>QEMU 直接启动（无 OpenSBI）<br>打印 "Hello, world!" 并关机<br>演示最基本的 OS 执行环境 | tg-sbi |
| **tg-ch2** | 批处理系统（Batch Processing）<br>特权级切换（U-mode ↔ S-mode）<br>Trap 处理（ecall / 异常）<br>上下文保存与恢复<br>系统调用（write / exit）<br>用户态 / 内核态<br>`sret` 返回指令 | 批处理操作系统<br>顺序加载运行多个用户程序<br>特权级切换和 Trap 处理框架<br>实现 write / exit 系统调用 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-syscall |
| **tg-ch3** | 多道程序（Multiprogramming）<br>任务控制块（TCB）<br>协作式调度（yield）<br>抢占式调度（Preemptive）<br>时钟中断（Clock Interrupt）<br>时间片轮转（Time Slice）<br>任务切换（Task Switch）<br>任务状态（Ready/Running/Finished）<br>clock_gettime 系统调用 | 多道程序与分时多任务<br>多程序同时驻留内存<br>协作式 + 抢占式调度<br>时钟中断与时间管理 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-syscall |
| **tg-ch4** | 虚拟内存（Virtual Memory）<br>Sv39 三级页表（Page Table）<br>地址空间隔离（Address Space）<br>页表项（PTE）与标志位<br>地址转换（VA → PA）<br>异界传送门（MultislotPortal）<br>ELF 加载与解析<br>堆管理（sbrk）<br>恒等映射（Identity Mapping）<br>内存保护（Memory Protection）<br>satp CSR | 引入 Sv39 虚拟内存<br>每个用户进程独立地址空间<br>跨地址空间上下文切换<br>进程隔离和内存保护 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-kernel-alloc<br>tg-kernel-vm<br>tg-syscall |
| **tg-ch5** | 进程（Process）<br>进程控制块（PCB）<br>进程标识符（PID）<br>fork（地址空间深拷贝）<br>exec（程序替换）<br>waitpid（等待子进程）<br>进程树 / 父子关系<br>初始进程（initproc）<br>Shell 交互式命令行<br>进程生命周期（Ready/Running/Zombie）<br>步幅调度（Stride Scheduling） | 引入进程管理<br>fork / exec / waitpid 系统调用<br>动态创建、替换、等待进程<br>Shell 交互式命令行 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-kernel-alloc<br>tg-kernel-vm<br>tg-syscall<br>tg-task-manage |
| **tg-ch6** | 文件系统（File System）<br>easy-fs 五层架构<br>SuperBlock / Inode / 位图<br>DiskInode（直接+间接索引）<br>目录项（DirEntry）<br>文件描述符表（fd_table）<br>文件句柄（FileHandle）<br>VirtIO 块设备驱动<br>MMIO（Memory-Mapped I/O）<br>块缓存（Block Cache）<br>硬链接（Hard Link）<br>open / close / read / write 系统调用 | 引入文件系统与 I/O<br>用户程序存储在磁盘镜像（fs.img）<br>VirtIO 块设备驱动<br>easy-fs 文件系统实现<br>文件打开 / 关闭 / 读写 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-kernel-alloc<br>tg-kernel-vm<br>tg-syscall<br>tg-task-manage<br>tg-easy-fs |
| **tg-ch7** | 进程间通信（IPC）<br>管道（Pipe）<br>环形缓冲区（Ring Buffer）<br>统一文件描述符（Fd 枚举）<br>信号（Signal）<br>信号集（SignalSet）<br>信号屏蔽字（Signal Mask）<br>信号处理函数（Signal Handler）<br>kill / sigaction / sigprocmask / sigreturn<br>命令行参数（argc / argv）<br>I/O 重定向（dup） | 进程间通信-管道 <br>异步事件通知（信号）<br>统一文件描述符抽象<br>信号发送 / 注册 / 屏蔽 / 返回 | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-kernel-alloc<br>tg-kernel-vm<br>tg-syscall<br>tg-task-manage<br>tg-easy-fs<br>tg-signal<br>tg-signal-impl |
| **tg-ch8** | 同步互斥（Sync&Mutex）<br>线程（Thread）/ 线程标识符（TID）<br>进程-线程分离<br>竞态条件（Race Condition）<br>临界区（Critical Section）<br>互斥（Mutual Exclusion）<br>互斥锁（Mutex：自旋锁 vs 阻塞锁）<br>信号量（Semaphore：P/V 操作）<br>条件变量（Condvar）<br>管程（Monitor：Mesa 语义）<br>线程阻塞与唤醒（wait queue）<br>死锁（Deadlock）/ 死锁四条件<br>银行家算法（Banker's Algorithm）<br>双层管理器（PThreadManager） | 进程-线程分离<br>同一进程内多线程并发<br>互斥锁（MutexBlocking）<br>信号量（Semaphore）<br>条件变量（Condvar）<br>线程阻塞与唤醒机制<br>死锁检测（练习） | tg-sbi<br>tg-linker<br>tg-console<br>tg-kernel-context<br>tg-kernel-alloc<br>tg-kernel-vm<br>tg-syscall<br>tg-task-manage<br>tg-easy-fs<br>tg-signal<br>tg-signal-impl<br>tg-sync |

### 表 2：tg-ch1 ~ tg-ch8 操作系统内核所依赖组件总体情况描述表

| 功能组件 | 所涉及核心知识点 | 主要完成功能 | 所依赖的组件 |
|:-----|:------------|:---------|:----------------------|
| **tg-sbi** | SBI（Supervisor Binary Interface）<br>console_putchar / console_getchar<br>系统关机（shutdown）<br>RISC-V 特权级（M/S-mode）<br>ecall 指令 | S→M 模式的 SBI 调用封装<br>字符输出 / 字符读取<br>系统关机<br>支持 nobios 直接操作 UART | 无 |
| **tg-console** | 控制台 I/O<br>格式化输出（print! / println!）<br>日志系统（Log Level）<br>自旋锁保护的全局控制台 | 可定制 print! / println! 宏<br>log::Log 日志实现<br>Console trait 抽象底层输出 | 无 |
| **tg-kernel-context** | 上下文（Context）<br>Trap 帧（Trap Frame）<br>寄存器保存与恢复<br>特权级切换<br>stvec / sepc / scause CSR<br>LocalContext（本地上下文）<br>ForeignContext（跨地址空间上下文）<br>异界传送门（MultislotPortal） | 用户/内核态切换上下文管理<br>LocalContext 结构<br>ForeignContext（含 satp）<br>MultislotPortal 跨地址空间执行 | 无 |
| **tg-kernel-alloc** | 内核堆分配器<br>伙伴系统（Buddy Allocation）<br>动态内存管理<br>#[global_allocator] | 基于伙伴算法的 GlobalAlloc<br>堆初始化（init）<br>物理内存转移（transfer） | 无 |
| **tg-kernel-vm** | 虚拟内存管理<br>页表（Page Table）<br>Sv39 分页（三级页表）<br>虚拟地址（VAddr）/ 物理地址（PAddr）<br>虚拟页号（VPN）/ 物理页号（PPN）<br>页表项（PTE）/ 页表标志位（VmFlags）<br>地址空间（AddressSpace）<br>PageManager trait<br>地址翻译（translate） | Sv39 页表管理<br>AddressSpace 地址空间抽象<br>虚实地址转换<br>页面映射（map / map_extern）<br>页表项操作 | 无 |
| **tg-syscall** | 系统调用（System Call）<br>系统调用号（SyscallId）<br>系统调用分发（handle）<br>系统调用结果（Done / Unsupported）<br>Caller 抽象<br>IO / Process / Scheduling / Clock /<br>Signal / Thread / SyncMutex trait 接口 | 系统调用 ID 与参数定义<br>trait 接口供内核实现<br>init_io / init_process / init_scheduling /<br>init_clock / init_signal /<br>init_thread / init_sync_mutex<br>支持 kernel / user feature | tg-signal-defs |
| **tg-task-manage** | 任务管理（Task Management）<br>调度（Scheduling）<br>进程管理器（PManager, proc feature）<br>双层管理器（PThreadManager, thread feature）<br>ProcId / ThreadId<br>就绪队列（Ready Queue）<br>Manage trait / Schedule trait<br>进程等待（wait / waitpid）<br>线程等待（waittid）<br>阻塞与唤醒（blocked / re_enque） | Manage 和 Schedule trait 抽象<br>proc feature：单层进程管理器（PManager）<br>thread feature：双层管理器（PThreadManager）<br>进程树 / 父子关系<br>线程阻塞 / 唤醒 | 无 |
| **tg-easy-fs** | 文件系统（File System）<br>SuperBlock / Inode / 位图（Bitmap）<br>DiskInode（直接+间接索引）<br>块缓存（Block Cache）<br>BlockDevice trait<br>文件句柄（FileHandle）<br>打开标志（OpenFlags）<br>管道（Pipe）/ 环形缓冲区<br>用户缓冲区（UserBuffer）<br>FSManager trait | easy-fs 五层架构实现<br>文件创建 / 读写 / 目录操作<br>块缓存管理<br>管道环形缓冲区实现<br>FSManager trait 抽象 | 无 |
| **tg-signal-defs** | 信号编号（SignalNo）<br>SIGKILL / SIGINT / SIGUSR1 等<br>信号动作（SignalAction）<br>信号集（SignalSet）<br>最大信号数（MAX_SIG） | 信号编号枚举定义<br>信号动作结构定义<br>信号集类型定义<br>为 tg-signal 和 tg-syscall 提供共用类型 | 无 |
| **tg-signal** | 信号处理（Signal Handling）<br>Signal trait 接口<br>add_signal / handle_signals<br>get_action_ref / set_action<br>update_mask / sig_return / from_fork<br>SignalResult（Handled / ProcessKilled） | Signal trait 接口定义<br>信号添加 / 处理 / 动作设置<br>屏蔽字更新 / 信号返回<br>fork 继承 | tg-kernel-context<br>tg-signal-defs |
| **tg-signal-impl** | SignalImpl 结构<br>已接收信号位图（received）<br>信号屏蔽字（mask）<br>信号处理中状态（handling）<br>信号动作表（actions）<br>信号处理函数调用<br>上下文保存与恢复 | Signal trait 的参考实现<br>信号接收位图管理<br>屏蔽字逻辑<br>处理状态和动作表 | tg-kernel-context<br>tg-signal |
| **tg-sync** | 互斥锁（Mutex trait: lock / unlock）<br>阻塞互斥锁（MutexBlocking）<br>信号量（Semaphore: up / down）<br>条件变量（Condvar: signal / wait_with_mutex）<br>等待队列（VecDeque\<ThreadId\>）<br>UPIntrFreeCell | MutexBlocking 阻塞互斥锁<br>Semaphore 信号量<br>Condvar 条件变量<br>通过 ThreadId 与调度器交互 | tg-task-manage |
| **tg-user** | 用户态程序（User-space App）<br>用户库（User Library）<br>系统调用封装（syscall wrapper）<br>用户堆分配器<br>用户态 print! / println! | 用户测试程序运行时库<br>系统调用封装<br>用户堆分配器<br>各章节测试用例（ch2~ch8） | tg-console<br>tg-syscall |
| **tg-checker** | 测试验证<br>输出模式匹配<br>正则表达式（Regex）<br>测试用例判定 | rCore-Tutorial CLI 测试输出检查工具<br>验证内核输出匹配预期模式<br>支持 --ch N 和 --exercise 模式 | 无 |
| **tg-linker** | 链接脚本（Linker Script）<br>内核内存布局（KernelLayout）<br>.text / .rodata / .data / .bss / .boot 段<br>入口点（boot0! 宏）<br>BSS 段清零 | 形成内核空间布局的链接脚本模板<br>用于 build.rs 工具构建 linker.ld<br>内核布局定位（KernelLayout::locate）<br>入口宏（boot0!）<br>段信息迭代 | 无 |
## License

Licensed under GNU GENERAL PUBLIC LICENSE, Version 3.0.
