# tg-signal-defs

Signal definitions for the rCore tutorial operating system.

## Overview

This crate provides the fundamental signal definitions used throughout the rCore tutorial kernel, including signal numbers and signal action structures compatible with POSIX-like systems.

## Features

- **SignalNo enum**: Complete set of standard Unix signals (SIGHUP through SIGSYS) plus real-time signals (SIGRT*)
- **SignalAction struct**: Signal handler configuration with handler address and signal mask
- **no_std compatible**: Designed for bare-metal kernel environments

## Signal Numbers

Standard signals (1-31):
- `SIGHUP`, `SIGINT`, `SIGQUIT`, `SIGILL`, `SIGTRAP`, `SIGABRT`
- `SIGBUS`, `SIGFPE`, `SIGKILL`, `SIGUSR1`, `SIGSEGV`, `SIGUSR2`
- `SIGPIPE`, `SIGALRM`, `SIGTERM`, `SIGSTKFLT`, `SIGCHLD`, `SIGCONT`
- `SIGSTOP`, `SIGTSTP`, `SIGTTIN`, `SIGTTOU`, `SIGURG`, `SIGXCPU`
- `SIGXFSZ`, `SIGVTALRM`, `SIGPROF`, `SIGWINCH`, `SIGIO`, `SIGPWR`, `SIGSYS`

Real-time signals (32-63):
- `SIGRTMIN` through `SIGRT31`

## Usage

```rust
use tg_signal_defs::{SignalNo, SignalAction, MAX_SIG};

// Create a signal action
let action = SignalAction {
    handler: handler_address,
    mask: 0,
};

// Use signal numbers
let sig = SignalNo::SIGINT;
```

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.
