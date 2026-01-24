# tg-kernel-context

Kernel context management for the rCore tutorial operating system.

## Overview

This crate provides context switching primitives for RISC-V based kernel development. It handles the low-level details of saving and restoring CPU state during context switches between kernel and user space.

## Features

- **LocalContext**: Thread context structure containing all general-purpose registers, program counter, and control flags
- **Context switching**: Assembly-based efficient context switch implementation
- **Foreign address space support**: Optional support for context switching across different address spaces (via `foreign` feature)
- **no_std compatible**: Designed for bare-metal kernel environments

## Usage

```rust
use tg_kernel_context::LocalContext;

// Create a user-mode context with entry point
let ctx = LocalContext::user(entry_point);

// Create a kernel thread context
let ctx = LocalContext::thread(entry_point, true);

// Execute the context (unsafe - modifies CPU state)
unsafe {
    let sstatus = ctx.execute();
}
```

## Features

- `foreign` - Enable support for context switching across different address spaces

## Safety

The `execute` method is unsafe as it directly manipulates critical CSRs (`sscratch`, `sepc`, `sstatus`, `stvec`) and performs a privilege level switch.

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
