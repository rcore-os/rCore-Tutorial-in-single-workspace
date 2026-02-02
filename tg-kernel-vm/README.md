# tg-kernel-vm

Kernel virtual memory management for the rCore tutorial operating system.

## Overview

This crate provides virtual memory management utilities for RISC-V based kernel development. It offers abstractions for managing address spaces, page tables, and physical page allocation.

## Features

- **AddressSpace**: High-level address space management
- **PageManager trait**: Abstract interface for physical page management
- **Page table integration**: Built on top of the `page-table` crate
- **no_std compatible**: Designed for bare-metal kernel environments

## Usage

```rust
use tg_kernel_vm::{AddressSpace, PageManager, page_table};
use page_table::{Pte, VmFlags, VmMeta, PPN};

// Implement the PageManager trait for your memory allocator
struct MyPageManager { /* ... */ }

impl<Meta: VmMeta> PageManager<Meta> for MyPageManager {
    fn new_root() -> Self { /* ... */ }
    fn root_ptr(&self) -> NonNull<Pte<Meta>> { /* ... */ }
    // ... other methods
}
```

## Core Abstractions

- `PageManager<Meta>` - Trait for physical page management including:
  - Root page table creation and access
  - Physical-to-virtual and virtual-to-physical address translation
  - Page allocation and deallocation
  - Ownership checking

## Dependencies

- `page-table` - Page table manipulation primitives

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
