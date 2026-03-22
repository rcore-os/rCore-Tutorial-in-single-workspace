# 用 QEMU + GDB 观察 ch3 多道程序与分时多任务的执行过程

本文档描述 5 个 GDB 调试演示场景，覆盖应用加载、特权级切换、系统调用、进程切换和时钟中断等 lec4 核心概念。

## 前置条件

- `qemu-system-riscv64`
- `riscv-none-elf-gdb`（或 `riscv-none-elf-gdb-py3`，推荐后者以使用 Python 扩展）
- 已编译：`cargo build`（需运行两次以获取完整符号信息）

## 快速启动

### 终端 A：启动 QEMU

```bash
cd tg-rcore-tutorial-ch3-show
bash scripts/launch-qemu-gdb.sh
```

### 终端 B：连接 GDB

```bash
cd tg-rcore-tutorial-ch3-show
riscv-none-elf-gdb-py3 -x gdb/ch3.gdb
```

加载 Python 扩展（可选但推荐）：

```text
(gdb) source scripts/gdb_ch3_stages.py
```

## 内存布局

| 地址范围 | 内容 | 特权级 |
|---------|------|-------|
| `< 0x80000000` | QEMU ROM / reset | - |
| `0x80000000 ~ 0x80200000` | M 态 SBI（`m_entry.asm`） | M |
| `0x80200000 ~ 0x80400000` | S 态内核（`src/main.rs`） | S |
| `0x80400000 + i * 0x200000` | 用户程序 app_i | U |

---

## 场景一：应用加载过程

**目的**：观察内核如何初始化 TCB、设置用户态入口地址和栈指针。

```text
(gdb) break rust_main
(gdb) continue
```

到达 `rust_main` 后，在加载循环处设断点：

```text
(gdb) list
(gdb) break main.rs:138       # for (i, app) in app_meta.iter()...
(gdb) continue
```

每次命中时观察：

```text
(gdb) print i
(gdb) print/x entry           # 用户程序入口地址（如 0x80400000）
(gdb) ch3stage                 # 应在 S 态内核
(gdb) continue                 # 继续加载下一个 app
```

**预期**：依次看到 app0 @ `0x80400000`、app1 @ `0x80600000`、... 被加载。

---

## 场景二：内核态到用户态切换（sret）

**目的**：观察 `execute()` 如何通过 `sret` 将 CPU 从 S 态切换到 U 态。

```text
(gdb) watch_priv_switch        # 在 execute_naked 入口设断点
(gdb) continue
```

到达后单步执行：

```text
(gdb) show_csrs                # 观察 sepc（将要跳转的用户 PC）、sstatus.SPP=U
(gdb) si                       # 单步进入裸汇编
```

关键观察点：

```text
(gdb) x/20i $pc               # 查看汇编：SAVE_ALL → 设置 stvec → LOAD_ALL → sret
(gdb) info registers sepc      # sret 后 PC 将变为 sepc 的值
(gdb) info registers sstatus   # SPP 位应为 0（将要进入 U 态）
```

用 `si` 单步到 `sret` 指令：

```text
(gdb) si                       # 执行 sret
(gdb) ch3stage                 # 现在应显示 "U 态用户程序 app0"
(gdb) info registers pc        # PC 应在 0x80400000 附近
```

**预期**：`sret` 前 SPP=0（目标 U 态），`sret` 后 PC 跳转到用户程序入口。

---

## 场景三：用户态到内核态切换（系统调用 ecall）

**目的**：观察用户程序执行 `ecall` 后如何陷入内核。

首先确定 stvec 的值：

```text
(gdb) info registers stvec     # trap 入口地址
```

在 trap 入口设断点：

```text
(gdb) break *<stvec 的值>      # 例如 break *0x80200abc
(gdb) continue
```

命中后观察：

```text
(gdb) show_csrs
(gdb) info registers scause    # 应为 8（UserEnvCall）
(gdb) info registers sepc      # 触发 ecall 的用户指令地址
(gdb) info registers a7        # 系统调用号（如 64=write, 93=exit, 124=yield）
(gdb) info registers a0 a1 a2  # 系统调用参数
(gdb) ch3stage                 # 应显示 S 态内核
```

**预期**：`scause=8` 表示 UserEnvCall，`a7` 包含系统调用号，`sepc` 指向用户态的 `ecall` 指令。

---

## 场景四：进程切换（轮转调度）

**目的**：观察从 app_i 切换到 app_j 的完整流程。

在主循环的任务切换处设断点：

```text
(gdb) break main.rs:257       # i = (i + 1) % index_mod
(gdb) continue
```

每次命中：

```text
(gdb) print i                  # 当前任务索引
(gdb) print remain             # 剩余未完成任务数
(gdb) continue                 # 切换到下一个任务
```

也可以观察连续的 execute/trap 循环：

```text
(gdb) break main.rs:183       # unsafe { tcb.execute() }
(gdb) continue
(gdb) print i                  # 正在执行哪个 app
(gdb) ch3stage                 # S 态，即将 sret 到 U 态
(gdb) continue                 # 执行 sret → 用户态运行 → trap 回来
```

**预期**：看到 `i` 在 0、1、2、... 之间循环，每个任务执行一个时间片后切换。

---

## 场景五：时钟中断的特权级切换

**目的**：观察时钟中断如何打断用户程序并触发抢占式调度。

在时钟中断处理分支设断点：

```text
(gdb) break main.rs:194       # Trap::Interrupt(Interrupt::SupervisorTimer)
(gdb) continue
```

命中后：

```text
(gdb) show_csrs
(gdb) info registers scause    # 应为 0x8000000000000005（S 态时钟中断）
(gdb) info registers sepc      # 被打断的用户指令地址
(gdb) print i                  # 被抢占的任务索引
(gdb) ch3stage                 # S 态内核（已从 U 态陷入）
```

继续到下一个 app 执行：

```text
(gdb) continue                 # 处理完后切换到下一个任务
```

**预期**：`scause` 最高位为 1（中断），低位为 5（SupervisorTimer）。`sepc` 指向被打断的用户代码地址（应在 `0x80400000+` 范围内）。

---

## 辅助命令速查

| 命令 | 说明 |
|------|------|
| `ch3stage` | 根据 PC 判断当前阶段（ROM/M-SBI/S-内核/U-用户） |
| `show_csrs` | 打印 sstatus/scause/sepc/stval/stvec/sscratch/sie/sip |
| `watch_priv_switch` | 在 execute_naked 设断点以观察 sret |
| `info registers` | 查看所有通用寄存器 |
| `x/Ni $pc` | 反汇编接下来 N 条指令 |
| `si` | 单步执行一条指令 |

## 与日常 `cargo run` 的关系

日常实验仍使用 `.cargo/config.toml` 的 runner（无 `-S`）。本目录下的脚本仅用于调试演示，不替换默认 runner。

## 备选：QEMU 指令日志

不使用 GDB 时，可在 QEMU 命令行添加 `-d in_asm` 查看执行的指令流（具体子选项见 `qemu-system-riscv64 -d help`）。
