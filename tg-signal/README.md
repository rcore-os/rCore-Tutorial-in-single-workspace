# tg-signal

Signal handling abstractions for the rCore tutorial operating system.

## Overview

This crate defines the `Signal` trait and related types for Unix-like signal handling in the rCore tutorial kernel. It provides the interface that signal implementations must follow.

## Features

- **Signal trait**: Abstract interface for signal management
- **SignalResult**: Enumeration of possible signal handling outcomes
- **Standard signal support**: Re-exports signal definitions from `tg-signal-defs`
- **no_std compatible**: Designed for bare-metal kernel environments

## Usage

```rust
use tg_signal::{Signal, SignalAction, SignalNo, SignalResult};

// Implement the Signal trait for your signal handler
impl Signal for MySignalHandler {
    fn from_fork(&mut self) -> Box<dyn Signal> { /* ... */ }
    fn add_signal(&mut self, signal: SignalNo) { /* ... */ }
    fn handle_signals(&mut self, ctx: &mut LocalContext) -> SignalResult { /* ... */ }
    // ... other methods
}
```

## Core Types

- `Signal` - Trait defining the signal handling interface
- `SignalResult` - Result type for signal handling operations (NoSignal, Handled, ProcessKilled, etc.)
- `SignalAction` - Signal handler configuration structure
- `SignalNo` - Signal number enumeration

## Related Crates

- `tg-signal-defs` - Signal number and action definitions
- `tg-signal-impl` - A concrete implementation of the Signal trait

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
