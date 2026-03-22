# ch4-show GDB 调试演示文档

本文档描述如何使用 `riscv-none-elf-gdb-py3` 配合 QEMU 调试 `tg-rcore-tutorial-ch4-show` 内核，
展示 Sv39 虚拟内存下的用户程序加载、特权级切换、页表管理、进程切换和时钟中断处理。

## 准备

```bash
# 终端 1：启动 QEMU（暂停等待 GDB 连接）
cd tg-rcore-tutorial-ch4-show
bash scripts/launch-qemu-gdb.sh

# 终端 2：启动 GDB
cd tg-rcore-tutorial-ch4-show
riscv-none-elf-gdb -x gdb/ch4.gdb
# 加载 Python 扩展
source scripts/gdb_ch4_stages.py
```

## 场景 1：应用加载（ELF 解析 + 地址空间映射）

**目标**：观察 ELF 文件解析、LOAD 段映射到用户地址空间的过程。

```gdb
# 在 Process::new 处设断点
break tg_rcore_tutorial_ch4_show::process::Process::new

continue   # 跳到内核加载第一个 ELF

# 查看当前阶段
ch4stage

# 单步进入 ELF 解析
next       # 跳过 ELF 头验证
next       # 进入 LOAD 段遍历循环

# 观察 ELF 段虚拟地址
print off_mem    # 段映射目标虚拟地址
print end_mem    # 段结束虚拟地址
print flags      # 权限标志

# 跳到进程创建完成，查看 satp
finish
show_satp        # 查看新进程的 satp
```

**观察要点**：
- 每个 LOAD 段被映射到用户虚拟地址（通常从 0x10000 开始）
- 权限标志包含 U（用户态可访问）
- 创建独立的页表（每个进程 satp 不同）

## 场景 2：内核态到用户态的特权级切换

**目标**：观察通过 MultislotPortal 从 S-mode 切换到 U-mode 的过程。

```gdb
# 方式 1：使用 Python 扩展
watch_priv_switch
continue

# 到达 execute_naked 后
show_csrs                          # 查看切换前的 CSR 状态
show_satp                          # 当前 satp（内核页表）
info registers sepc sstatus sp     # 关键寄存器

# 单步执行到 sret
si
si   # 反复单步直到 sret

# 方式 2：在 schedule 的 ctx.execute() 处设断点
break schedule
continue
next   # 跳到 ctx.execute(portal, ())
# 执行前
show_satp    # 此时 satp 指向内核页表
# si 进入 execute
si
show_satp    # 注意 satp 变化到用户页表
```

**观察要点**：
- sret 前：`sstatus.SPP=U`（目标特权级为用户态），`sepc` 指向用户程序入口
- sret 后：PC 跳转到用户虚拟地址，satp 切换到用户页表
- 传送门页面同时映射在内核和用户地址空间的相同虚拟地址

## 场景 3：用户态到内核态的特权级切换（系统调用）

**目标**：观察 ecall 触发 trap，通过传送门返回内核地址空间。

```gdb
# 在系统调用处理处设断点
break schedule
continue
# 到达 schedule 后，让它执行一轮
next   # 初始化传送门
# 设条件断点：在 UserEnvCall 处理分支
break *schedule+<offset>   # 或按行号设
continue

# 到达系统调用处理后
show_csrs          # scause 应为 Exception(UserEnvCall)
print id           # 系统调用号
print args         # 系统调用参数

# 观察地址翻译
# IO::write 中用 translate() 将用户 VA → PA
ch4stage           # 确认在 S 态内核中
```

**观察要点**：
- `scause` = 8（UserEnvCall），表示 ecall 系统调用
- 内核通过 `address_space.translate()` 将用户指针翻译为物理地址
- 系统调用返回前 `sepc += 4`（跳过 ecall 指令）

## 场景 4：进程页表内容 + 包含页表切换的进程切换

**目标**：观察不同进程的页表内容差异和 satp 切换。

```gdb
# 在进程退出并切换到下一个进程时设断点
# 串口输出中的 [LEC5-LAB4] kp=process_switch 标签标记了切换点

# 方法 1：在 EXIT 处理后设断点
break schedule
continue
# 用 next 跳到 Id::EXIT 分支

# 查看当前进程的 satp
show_satp                # 退出进程的页表
# 进程移除后
next
show_satp                # 新的当前进程的页表（satp 已变化）

# 方法 2：使用 GDB 打印 AddressSpace
# 在 Process::new 中 address_space 创建完成后查看页表
print address_space      # Rust Debug 格式显示页表项

# 手动检查页表项（假设 root_ppn 已知）
# 根页表物理地址 = root_ppn << 12
x/512gx (0x813b4 << 12)  # 查看根页表 512 项

# 检查二级页表
# 从根页表非零项中提取 PPN，递归查看
```

**观察要点**：
- 每个进程的 satp 中 PPN 不同（独立的根页表）
- 进程切换 = 更换 satp + 恢复上下文
- 页表项中的标志位：V(有效) R(读) W(写) X(执行) U(用户) G(全局)
- 传送门页表项在所有进程的页表中都是相同的（共享映射）

## 场景 5：时钟中断导致的用户态/内核态切换

**目标**：观察 SupervisorTimer 中断触发抢占。

```gdb
# 在 SupervisorTimer 分支设断点
# 需要知道对应的代码行号
break schedule    # 先到 schedule
continue
# 在循环内设断点：match 的 SupervisorTimer 分支

# 或者用条件断点
# 当 scause 的最高位为 1（中断）且 code 为 5（STimer）时触发

# 到达断点后
show_csrs
# scause 应显示 [Interrupt: STimer]
# sepc 指向被打断的用户程序指令地址
# sstatus.SPP = U（从用户态被中断）

ch4stage       # 确认在 S 态内核处理中断
```

**观察要点**：
- 时钟中断异步发生，不可预测具体时机
- `scause` 最高位为 1 表示中断（区别于异常的最高位为 0）
- 内核处理完中断后通过传送门返回用户态继续执行
- ch4-show 中时钟中断不切换进程（保持顺序执行语义）

## 实用 GDB 命令速查

| 命令 | 说明 |
|------|------|
| `ch4stage` | 显示当前阶段（ROM/M-SBI/S-内核/U-用户） |
| `show_csrs` | 打印所有关键 S 态 CSR（含 satp 解析） |
| `show_satp` | 解析 satp 寄存器（模式/ASID/PPN） |
| `watch_priv_switch` | 在 execute_naked 设断点 |
| `info registers` | 查看通用寄存器 |
| `x/512gx <addr>` | 查看页表内容（512 个 8 字节项） |
| `si` | 单条指令步进 |
| `ni` | 单条指令步进（不进入函数） |
| `bt` | 查看调用栈 |
