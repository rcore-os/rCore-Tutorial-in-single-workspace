# tg-syscall

[![Crates.io](https://img.shields.io/crates/v/tg-syscall.svg)](https://crates.io/crates/tg-syscall)
[![Documentation](https://docs.rs/tg-syscall/badge.svg)](https://docs.rs/tg-syscall)
[![License](https://img.shields.io/crates/l/tg-syscall.svg)](LICENSE)

System call definitions and interfaces for the rCore tutorial operating system.

## Overview

This crate provides system call number definitions and a framework for implementing system calls in the rCore tutorial kernel. System call numbers are generated from Musl Libc for RISC-V source code.

## Features

- **System call number definitions**: Generated from Musl Libc for RISC-V
- **Syscall framework**: Trait-based system call implementation
- **User and kernel modes**: Separate features for user-space and kernel-space usage
- **no_std compatible**: Designed for bare-metal environments

## Usage

### Kernel side (with `kernel` feature)

```rust
use tg_syscall::{Caller, SyscallId, SyscallResult};

// Initialize syscall handlers
tg_syscall::init_io(&my_io_impl);
tg_syscall::init_process(&my_process_impl);
tg_syscall::init_scheduling(&my_sched_impl);

// Handle syscalls
let result = tg_syscall::handle(caller, id, args);
```

### User side (with `user` feature)

```rust
use tg_syscall::{write, read, exit};

// Make system calls
write(STDOUT, buffer);
exit(0);
```

## Features

- `kernel` - Enable kernel-side syscall handling interfaces
- `user` - Enable user-space syscall wrappers

## Supported System Calls

Standard POSIX-compatible system calls including:
- I/O: `read`, `write`, `open`, `close`
- Process: `fork`, `exec`, `exit`, `wait`, `getpid`
- Signal: `kill`, `sigaction`, `sigprocmask`, `sigreturn`
- Thread: `thread_create`, `gettid`, `waittid`
- Scheduling: `sched_yield`
- Time: `clock_gettime`
- Synchronization: `semaphore_*`, `mutex_*`, `condvar_*`

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
