# tg-sync

Synchronization primitives for the rCore tutorial operating system.

## Overview

This crate provides essential synchronization primitives for kernel development, including mutexes, semaphores, and condition variables suitable for a teaching operating system.

## Features

- **Mutex**: Basic mutual exclusion lock
- **MutexBlocking**: Blocking mutex with thread scheduling integration
- **Semaphore**: Counting semaphore for resource management
- **Condvar**: Condition variable for thread synchronization
- **UPIntrFreeCell**: Uniprocessor interrupt-free cell for safe interior mutability
- **no_std compatible**: Designed for bare-metal kernel environments

## Usage

```rust
use tg_sync::{Mutex, MutexBlocking, Semaphore, Condvar};
use tg_sync::{UPIntrFreeCell, UPIntrRefMut};

// Use mutex for mutual exclusion
let mutex = MutexBlocking::new();
mutex.lock();
// critical section
mutex.unlock();

// Use semaphore for resource counting
let sem = Semaphore::new(3);
sem.down();
// use resource
sem.up();

// Use condition variable
let condvar = Condvar::new();
condvar.wait();
condvar.signal();
```

## Core Types

- `Mutex` - Basic spinlock-based mutex
- `MutexBlocking` - Mutex with thread blocking support
- `Semaphore` - Counting semaphore
- `Condvar` - Condition variable
- `UPIntrFreeCell` - Interrupt-safe cell for uniprocessor systems

## Dependencies

- `tg-task-manage` - For thread scheduling integration

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
