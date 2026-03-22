# 用 QEMU + GDB 观察启动链：ROM → SBI（M 态）→ 内核（S 态）

本说明仅使用 **单一 ELF**（`cargo build` 产物）：其中 **M 态** 代码来自依赖 `tg-rcore-tutorial-sbi`（`nobios`），**S 态** 为本 crate 的 `_start`。链接布局见根目录 `build.rs` 中的 `LINKER_SCRIPT`（`M_BASE_ADDRESS = 0x80000000`，`S_BASE_ADDRESS = 0x80200000`）。

## 你需要

- `qemu-system-riscv64`
- `riscv-none-elf-gdb`（或带 Python 的 `riscv-none-elf-gdb-py3`）

## 步骤

### 1. 终端 A：启动 QEMU（GDB stub + 上电暂停）

在 **`tg-rcore-tutorial-ch1-show` 根目录**：

```bash
bash scripts/launch-qemu-gdb.sh
```

release 构建：

```bash
PROFILE=release bash scripts/launch-qemu-gdb.sh
```

脚本会打印 **ELF 路径** 与 **GDB 连接方式**。QEMU 使用 `-s`（默认 `localhost:1234`）与 `-S`（启动后冻结 CPU）。

### 2. 终端 B：连接 GDB

仍在 crate **根目录**：

```bash
riscv-none-elf-gdb -x gdb/boot.gdb
```

若使用 **release** ELF，请先编辑 `gdb/boot.gdb` 中的 `file` 行，或手动：

```text
riscv-none-elf-gdb
(gdb) file target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch1-show
(gdb) target remote :1234
```

### 3. 三段「第一条指令」建议观察顺序

| 阶段 | 典型 PC 范围 | 说明 | 源码对照 |
|------|----------------|------|----------|
| QEMU 内置复位 / ROM | 连接后见 `info reg pc`（常为 `0x1000` 附近） | 非本课程仓库代码；用 `x/16i $pc`、`si` | QEMU 模拟器内置 |
| SBI（M 态） | `_m_start` @ `0x80000000` | `break _m_start`，`continue` 或从 ROM 单步到跳转后 | `../tg-rcore-tutorial-sbi/src/m_entry.asm` |
| rcore（S 态） | `_start` @ `0x80200000` | `break _start`；或在 `_m_start` 末 `mret` 前看 `mepc` | `src/main.rs` 中 `_start` |

**M → S 切换**：`m_entry.asm` 末尾 `mret` 进入 `mepc` 指向的 S 态入口（即 `_start`）。可用 `info registers mepc mstatus` 对照课堂讲解。

### 4.（可选）Python 命令 `bootstage`

使用 `riscv-none-elf-gdb-py3`，在 GDB 内：

```text
source scripts/gdb_bootstage.py
bootstage
```

会根据当前 `$pc` 打印粗阶段说明（ROM / M 态 / S 态）。

### 5.（可选）大 ELF 加速符号加载

若符号加载很慢，可对 ELF 运行 `riscv-none-elf-gdb-add-index`（工具链提供）后再用 GDB 打开。

## 备份：仅 QEMU 日志

不上 GDB 时，可对 `qemu-system-riscv64` 增加 `-d in_asm`（具体子选项以 `qemu-system-riscv64 -d help` 为准），截取启动最初若干条指令用于讲义。

## 与日常 `cargo run` 的关系

日常实验仍使用 `.cargo/config.toml` 的 **runner**（无 `-S`）。本目录下的脚本 **仅用于调试演示**，不替换默认 runner。
