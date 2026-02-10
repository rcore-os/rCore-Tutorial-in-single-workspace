//! 处理器管理模块
//!
//! 与第五章完全相同：PROCESSOR 全局管理器 + ProcManager 进程管理器。
//! 调度算法仍为简单的 FIFO/RR。

use crate::process::Process;
use alloc::collections::{BTreeMap, VecDeque};
use core::cell::UnsafeCell;
use tg_task_manage::{Manage, PManager, ProcId, Schedule};

/// 处理器全局管理器
pub struct Processor {
    inner: UnsafeCell<PManager<Process, ProcManager>>,
}

unsafe impl Sync for Processor {}

impl Processor {
    /// 创建新的处理器管理器
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(PManager::new()),
        }
    }

    /// 获取内部 PManager 的可变引用
    #[inline]
    pub fn get_mut(&self) -> &mut PManager<Process, ProcManager> {
        unsafe { &mut (*self.inner.get()) }
    }
}

/// 全局处理器管理器实例
pub static PROCESSOR: Processor = Processor::new();

/// 进程管理器（FIFO 调度）
pub struct ProcManager {
    /// 所有进程实体的映射表
    tasks: BTreeMap<ProcId, Process>,
    /// 就绪队列
    ready_queue: VecDeque<ProcId>,
}

impl ProcManager {
    /// 创建新的进程管理器
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            ready_queue: VecDeque::new(),
        }
    }
}

impl Manage<Process, ProcId> for ProcManager {
    /// 插入新进程
    #[inline]
    fn insert(&mut self, id: ProcId, task: Process) {
        self.tasks.insert(id, task);
    }
    /// 根据 PID 获取进程
    #[inline]
    fn get_mut(&mut self, id: ProcId) -> Option<&mut Process> {
        self.tasks.get_mut(&id)
    }
    /// 删除进程
    #[inline]
    fn delete(&mut self, id: ProcId) {
        self.tasks.remove(&id);
    }
}

impl Schedule<ProcId> for ProcManager {
    /// 加入就绪队列尾部
    fn add(&mut self, id: ProcId) {
        self.ready_queue.push_back(id);
    }
    /// 从就绪队列头部取出
    fn fetch(&mut self) -> Option<ProcId> {
        self.ready_queue.pop_front()
    }
}
