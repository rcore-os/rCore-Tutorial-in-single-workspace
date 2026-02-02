# tg-task-manage

[![Crates.io](https://img.shields.io/crates/v/tg-task-manage.svg)](https://crates.io/crates/tg-task-manage)
[![Documentation](https://docs.rs/tg-task-manage/badge.svg)](https://docs.rs/tg-task-manage)
[![License](https://img.shields.io/crates/l/tg-task-manage.svg)](LICENSE)

任务管理模块，为 rCore 教学操作系统提供任务和进程管理功能，包括调度和关系管理。

## 功能特性

### 任务 ID 类型
自增不回收，任务对象之间的关系通过 ID 类型来实现：
- `ProcId` - 进程 ID
- `ThreadId` - 线程 ID
- `CoroId` - 协程 ID

### 任务对象管理 (`Manage` trait)
对标数据库增删改查操作：
- `insert` - 插入任务
- `delete` - 删除任务
- `get_mut` - 获取可变引用

### 任务调度 (`Schedule` trait)
队列中保存需要调度的任务 ID：
- `add` - 任务进入调度队列
- `fetch` - 从调度队列中取出一个任务

### 任务关系封装
使得 `PCB`、`TCB` 内部更加简洁：
- `ProcRel` - 进程与其子进程之间的关系
- `ProcThreadRel` - 进程、子进程以及其地址空间内的线程之间的关系

## Features

- `proc` - 启用进程管理功能
- `thread` - 启用线程管理功能

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

