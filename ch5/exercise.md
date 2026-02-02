# chapter5 练习

## 编程作业

### 关于之前的 syscall

你仍需要迁移上一章的 `mmap` `munmap` 以适应新的进程结构。

**从本章节开始，不再要求维护 `trace` 这一系统调用**。

### 进程创建：spawn 系统调用

大家一定好奇过为啥进程创建要用 fork + exec 这么一个奇怪的系统调用，就不能直接搞一个新进程吗？
思而不学则殆，我们就来试一试！这章的编程练习请大家实现一个完全 DIY 的系统调用 spawn，用以创建一个新进程。

spawn 系统调用定义（[标准 spawn 看这里](https://man7.org/linux/man-pages/man3/posix_spawn.3.html)）：

```rust
fn spawn(&self, _caller: Caller, path: usize, count: usize) -> isize
```

- syscall ID: 400
- 功能：新建子进程，使其执行目标程序。
- 参数：path 目标程序路径，count 路径长度
- 说明：成功返回子进程 id，否则返回 -1。
- 可能的错误：
  - 无效的文件名。

#### 注意

虽然测例很简单，但提醒读者 spawn **不必** 像 fork 一样复制父进程的地址空间。

### stride 调度算法

ch3 中我们实现的调度算法十分简单。现在我们要为我们的 OS 实现一种带优先级的调度算法：stride 调度算法。

算法描述如下：

1. 为每个进程设置一个当前 stride，表示该进程当前已经运行的"长度"。另外设置其对应的 pass 值（只与进程的优先权有关系），表示对应进程在调度后，stride 需要进行的累加值。

2. 每次需要调度时，从当前 runnable 态的进程中选择 stride 最小的进程调度。对于获得调度的进程 P，将对应的 stride 加上其对应的步长 pass。

3. 一个时间片后，回到上一步骤，重新调度当前 stride 最小的进程。

可以证明，如果令 `P.pass = BigStride / P.priority`，其中 `P.priority` 表示进程的优先权（大于 1），而 BigStride 表示一个预先定义的大常数，则该调度方案为每个进程分配的时间将与其优先级成正比。

其他实验细节：

- stride 调度要求进程优先级 >= 2，所以设定进程优先级 <= 1 会导致错误。
- 进程初始 stride 设置为 0 即可。
- 进程初始优先级设置为 16。

为了实现该调度算法，内核还要增加 set_priority 系统调用：

```rust
fn set_priority(&self, _caller: Caller, prio: isize) -> isize
```

- syscall ID：140
- 设置当前进程优先级为 prio
- 参数：prio 进程优先级，要求 prio >= 2
- 返回值：如果输入合法则返回 prio，否则返回 -1

### HINT

- 你可以在 `Process` 中添加新字段（如 `stride`、`priority`）来支持优先级调度
- 为了减少整数除的误差，BigStride 一般需要很大，但为了不至于发生溢出反转现象，或许选择一个适中的数即可，当然能进行溢出处理就更好了。
- stride 算法要找到 stride 最小的进程，使用优先级队列是效率不错的办法，但是我们的实验测例很简单，所以效率完全不是问题。事实上，很推荐使用暴力扫一遍的办法找最小值。
- 注意设置进程的初始优先级。

### 实验要求

- 在 tg-ch5 目录下完成实验。
- 目录结构说明：

```
tg-ch5/
├── Cargo.toml（内核配置文件）
├── src/（内核源代码，需要修改）
│   ├── main.rs（内核主函数，包括系统调用接口实现）
│   ├── process.rs（进程结构）
│   └── processor.rs（进程管理器和调度器）
└── tg-user/（用户程序，运行时自动拉取，无需修改）
    └── src/bin（测试用例）
```

> **说明**：
> - `tg-user` 会在运行时自动拉取到 `tg-ch5/tg-user` 目录下
> - 只需修改 `tg-ch5/src/` 目录下的内核代码

- 运行练习测例：
```bash
cargo run --features exercise
```
然后在终端中输入 `ch5_usertest` 运行，这个测例打包了所有你需要通过的测例。
你也可以通过修改这个文件调整本地测试的内容, 或者单独运行某测例来纠正特定的错误。

- 测试练习测例：
```bash
./test.sh exercise
```

### 说明

- 从本章开始，你的内核必须前向兼容，需要能通过前一章的所有测例（除了 `ch3_trace` 和 `ch4_trace`）