# 用 QEMU + GDB 观察特权级切换：S 态 → U 态 → Trap → S 态

本说明使用 `cargo build` 产物（单一 ELF），其中 M 态代码来自 `tg-rcore-tutorial-sbi`（nobios），S 态为本 crate 的 `_start`，U 态为内嵌的用户程序（加载到 `0x80400000`）。

## 你需要

- `qemu-system-riscv64`
- `riscv-none-elf-gdb`（或带 Python 的 `riscv-none-elf-gdb-py3`）

## 步骤

### 1. 终端 A：启动 QEMU（GDB stub + 上电暂停）

在 **`tg-rcore-tutorial-ch2-show` 根目录**：

```bash
bash scripts/launch-qemu-gdb.sh
```

QEMU 使用 `-s`（默认 `localhost:1234`）与 `-S`（启动后冻结 CPU）。

### 2. 终端 B：连接 GDB

仍在 crate **根目录**：

```bash
riscv-none-elf-gdb -x gdb/ch2-trap.gdb
```

### 3. 观察启动链（ROM → M 态 → S 态）

| 阶段 | 典型 PC 范围 | 说明 | 源码对照 |
|------|------------|------|---------|
| QEMU ROM | `0x1000` 附近 | 非本课程代码 | QEMU 内置 |
| SBI（M 态）| `_m_start` @ `0x80000000` | `break _m_start` 后 `continue` | `../tg-rcore-tutorial-sbi/src/m_entry.asm` |
| 内核（S 态）| `_start` @ `0x80200000` | `break _start` | `src/main.rs` |

**M → S 切换**：`m_entry.asm` 末尾 `mret` 进入 `mepc` 指向的 S 态入口。可用 `info registers mepc mstatus` 对照。

### 4. 观察 S 态 → U 态切换（内核加载应用 → sret）

```text
(gdb) continue
# 停在 run_batch
(gdb) continue
# 停在 *0x80400000（用户程序入口）
```

此时 CPU 已通过 `sret` 从 S 态切换到 U 态。观察：

```text
(gdb) trapstage
# 应显示 "U-mode user app"
(gdb) info registers sstatus
# SPP 位应为 0（表示从 U 态 trap 时会回到 U 态）
(gdb) info registers sepc
# sepc = 用户程序入口地址
```

**关键机制**：`LocalContext::execute()` → `execute_naked()` → `sret`
- `sstatus.SPP = 0`（User）：使 `sret` 后进入 U 态
- `sepc = app_base`：`sret` 后 PC 跳转到用户程序入口
- `sscratch` 保存 `LocalContext` 指针，用于 trap 时恢复内核栈

### 5. 观察 U 态 → S 态切换（ecall / 异常 → trap）

用户程序执行 `ecall`（系统调用）时：

```text
(gdb) continue
# 停在 handle_syscall
(gdb) trapstage
# 应显示 "S-mode kernel"，scause = UserEnvCall (8)
(gdb) info registers scause sepc sstatus
# scause = 8 (UserEnvCall)
# sepc = ecall 指令的地址
# sstatus.SPP = 0（从 U 态陷入）
```

**关键机制**：
1. 用户执行 `ecall` → 硬件自动：PC → `stvec`，`sepc` ← 当前 PC，`scause` ← 8
2. `execute_naked` 的 trap 入口 `1:`：`csrrw sp, sscratch, sp` 切换到内核栈
3. `SAVE_ALL` 保存用户寄存器到 `LocalContext`
4. 恢复调度上下文，`ret` 返回到 `ctx.execute()` 之后

### 6. 观察 sepc += 4（INV-SYSCALL-SEPC）

在 `handle_syscall` 中单步：

```text
(gdb) break handle_syscall
(gdb) continue
# 停在 handle_syscall
(gdb) info registers sepc
# sepc = ecall 指令地址（如 0x804000xx）
# 执行 ctx.move_next() 后：
(gdb) next
# ...
(gdb) info registers sepc
# sepc = 原地址 + 4（跳过 ecall）
```

这保证 `sret` 返回用户后从 ecall 的**下一条**指令继续执行。

### 7. 观察用户异常（StoreFault / IllegalInstruction）

ch2 用户程序 `01store_fault` 会触发访存错误：

```text
(gdb) continue
# 内核输出 "app was killed because of ..."
(gdb) trapstage
# scause = StoreFault (7) 或 IllegalInstruction (2)
# stval = 触发异常的地址/指令
```

### 8.（可选）Python 命令 `trapstage`

使用 `riscv-none-elf-gdb-py3`，在 GDB 内：

```text
source scripts/gdb_trap_stage.py
trapstage
```

会根据 `$pc` 打印当前阶段（ROM / M 态 / S 态内核 / U 态用户），并显示 `sstatus`、`scause`、`sepc` 等 CSR 值。

### 9. 完整特权级切换序列图

```
M-mode SBI           S-mode Kernel           U-mode App
    |                     |                      |
    |-- mret ------------>|                      |
    |                     |-- sret ------------->|
    |                     |                      |-- ecall
    |                     |<-- trap -------------|
    |                     |-- handle_syscall     |
    |                     |-- move_next (sepc+4) |
    |                     |-- sret ------------->|
    |                     |                      |-- ecall (exit)
    |                     |<-- trap -------------|
    |                     |-- load next app      |
    |                     |-- fence.i            |
    |                     |-- sret ------------->| (next app)
```

## 与日常 `cargo run` 的关系

日常实验使用 `.cargo/config.toml` 的 runner（无 `-S`）。本目录下的脚本**仅用于调试演示**。
