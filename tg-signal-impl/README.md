# tg-signal-impl

A concrete signal handling implementation for the rCore tutorial operating system.

## Overview

This crate provides `SignalImpl`, a complete implementation of the `Signal` trait from `tg-signal`. It handles signal queuing, masking, delivery, and user-space signal handler invocation.

## Features

- **Signal queuing**: Bitmap-based received signal tracking
- **Signal masking**: Support for blocking signals via mask
- **User signal handlers**: Save/restore context for user-space signal handlers
- **Process control signals**: Support for SIGSTOP/SIGCONT process suspension
- **Default actions**: Built-in default actions for unhandled signals
- **no_std compatible**: Designed for bare-metal kernel environments

## Usage

```rust
use tg_signal_impl::SignalImpl;
use tg_signal::Signal;

// Create a new signal handler
let mut signals = SignalImpl::new();

// Add a signal
signals.add_signal(SignalNo::SIGINT);

// Handle pending signals
let result = signals.handle_signals(&mut context);
```

## Architecture

- `SignalImpl` - Main signal management structure
- `SignalSet` - Bitmap-based signal set for efficient signal tracking
- `HandlingSignal` - Enum tracking current signal handling state (Frozen or UserSignal)
- `DefaultAction` - Default signal actions (terminate, ignore, stop, continue)

## Signal Handling Flow

1. Signals are added via `add_signal()`
2. `handle_signals()` checks for pending unmasked signals
3. For user handlers: saves context and redirects execution to handler
4. `sig_return()` restores the original context after handler completes

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
