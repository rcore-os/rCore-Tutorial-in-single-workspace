# tg-easy-fs

A simple filesystem implementation for the rCore tutorial operating system.

## Overview

This crate provides a lightweight filesystem (EasyFS) implementation designed for educational purposes. It features a classic Unix-like filesystem structure with inodes, block caching, and a simple yet functional virtual filesystem interface.

## Features

- **Block-based storage**: Uses 512-byte blocks as the fundamental storage unit
- **Inode-based structure**: Unix-like inode system for file metadata management
- **Block caching**: Efficient block cache layer for improved I/O performance
- **Bitmap allocation**: Bitmap-based block and inode allocation
- **no_std compatible**: Designed for bare-metal kernel environments

## Usage

This crate is primarily used within the rCore tutorial kernel (ch6+) for file operations.

```rust
use tg_easy_fs::{BlockDevice, EasyFileSystem, Inode};

// Implement BlockDevice trait for your storage device
// Then create and use the filesystem
```

## Architecture

- `BlockDevice` - Trait for block device abstraction
- `EasyFileSystem` - Main filesystem structure
- `Inode` - Virtual filesystem node interface
- `BlockCache` - Block caching layer

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
