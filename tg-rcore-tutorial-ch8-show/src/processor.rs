//! 处理器与调度模块
//!
//! ## 与第七章的区别
//!
//! 第七章使用 `PManager`（进程管理器）作为全局处理器类型；
//! 第八章使用 `PThreadManager`（进程 + 线程双层管理器），
//! 支持一个进程拥有多个线程。
//!
//! ## 核心类型
//!
//! - `ProcessorInner = PThreadManager<Process, Thread, ThreadManager, ProcManager>`
//! - `ThreadManager`：管理线程实体和就绪队列
//! - `ProcManager`：管理进程实体
//!
//! 教程阅读建议：
//!
//! - 先看 `ProcessorInner` 类型别名：先建立“统一入口，双层实体”的心智模型；
//! - 再看 `ThreadManager` 与 `ProcManager` 的 `Manage` 实现：理解两层对象如何独立维护；
//! - 最后看 `Schedule<ThreadId>`：明确调度粒度已经从进程切换为线程。

use crate::process::{Process, Thread};
use alloc::collections::{BTreeMap, VecDeque};
use core::cell::UnsafeCell;
use tg_task_manage::{Manage, PThreadManager, ProcId, Schedule, ThreadId};

/// 处理器内部类型（双层管理器）
pub type ProcessorInner = PThreadManager<Process, Thread, ThreadManager, ProcManager>;

/// 全局处理器包装（通过 `UnsafeCell` 允许内部可变）
pub struct Processor {
    inner: UnsafeCell<ProcessorInner>,
}

unsafe impl Sync for Processor {}

impl Processor {
    /// 创建新处理器
    pub const fn new() -> Self {
        Self { inner: UnsafeCell::new(PThreadManager::new()) }
    }

    /// 获取内部可变引用
    #[inline]
    pub fn get_mut(&self) -> &mut ProcessorInner {
        unsafe { &mut (*self.inner.get()) }
    }
}

/// 全局处理器实例
pub static PROCESSOR: Processor = Processor::new();

/// 线程管理器
///
/// 维护所有线程实体和就绪队列。
/// 使用 FIFO 调度策略。
pub struct ThreadManager {
    /// 线程实体表（TID → Thread）
    tasks: BTreeMap<ThreadId, Thread>,
    /// 就绪队列
    ready_queue: VecDeque<ThreadId>,
}

impl ThreadManager {
    /// 创建空的线程管理器
    pub fn new() -> Self {
        Self { tasks: BTreeMap::new(), ready_queue: VecDeque::new() }
    }
}

impl Manage<Thread, ThreadId> for ThreadManager {
    /// 插入线程实体
    #[inline]
    fn insert(&mut self, id: ThreadId, task: Thread) { self.tasks.insert(id, task); }
    /// 获取线程可变引用
    #[inline]
    fn get_mut(&mut self, id: ThreadId) -> Option<&mut Thread> { self.tasks.get_mut(&id) }
    /// 删除线程实体
    #[inline]
    fn delete(&mut self, id: ThreadId) { self.tasks.remove(&id); }
}

impl Schedule<ThreadId> for ThreadManager {
    /// 加入就绪队列
    fn add(&mut self, id: ThreadId) { self.ready_queue.push_back(id); }
    /// 取出下一个就绪线程
    fn fetch(&mut self) -> Option<ThreadId> { self.ready_queue.pop_front() }
}

/// 进程管理器
///
/// 维护所有进程实体（PID → Process）。
pub struct ProcManager {
    procs: BTreeMap<ProcId, Process>,
}

impl ProcManager {
    /// 创建空的进程管理器
    pub fn new() -> Self {
        Self { procs: BTreeMap::new() }
    }
}

impl Manage<Process, ProcId> for ProcManager {
    /// 插入进程实体
    #[inline]
    fn insert(&mut self, id: ProcId, item: Process) { self.procs.insert(id, item); }
    /// 获取进程可变引用
    #[inline]
    fn get_mut(&mut self, id: ProcId) -> Option<&mut Process> { self.procs.get_mut(&id) }
    /// 删除进程实体
    #[inline]
    fn delete(&mut self, id: ProcId) { self.procs.remove(&id); }
}
