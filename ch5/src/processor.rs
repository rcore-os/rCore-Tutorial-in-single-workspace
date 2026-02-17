//! 处理器管理模块
//!
//! 定义 `PROCESSOR` 全局变量和 `ProcManager` 进程管理器。
//!
//! ## 设计思路
//!
//! 进程管理分为两部分：
//! - `PROCESSOR`：封装 `PManager`，提供全局访问接口，管理当前运行的进程
//! - `ProcManager`：实现 `Manage` 和 `Schedule` trait，负责进程的存储和调度
//!
//! ## 调度算法
//!
//! 当前使用简单的 **先进先出（FIFO）** / **时间片轮转（RR）** 调度：
//! - `add`：将进程加入就绪队列尾部
//! - `fetch`：从就绪队列头部取出下一个要执行的进程
//!
//! 练习题要求实现 **stride 调度算法**，需要修改此模块。
//!
//! 教程阅读建议：
//!
//! - 先看 `ProcManager`：理解“存储结构(BTreeMap) + 调度结构(VecDeque)”双结构搭配；
//! - 再看 `Manage` 与 `Schedule` trait：理解抽象层如何为后续替换调度算法留接口；
//! - 最后结合 `ch5/src/main.rs` 中对 `PROCESSOR` 的调用观察状态流转。

use crate::process::Process;
use alloc::collections::{BTreeMap, VecDeque};
use core::cell::UnsafeCell;
use tg_task_manage::{Manage, PManager, ProcId, Schedule};

/// 处理器全局管理器
///
/// 封装 `PManager<Process, ProcManager>`，通过 `UnsafeCell` 提供内部可变性。
/// 在单核环境下是安全的，因为不会出现并发访问。
pub struct Processor {
    inner: UnsafeCell<PManager<Process, ProcManager>>,
}

unsafe impl Sync for Processor {}

impl Processor {
    /// 创建新的处理器管理器（编译期常量初始化）
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

/// 进程管理器
///
/// 负责管理所有进程实体和调度队列：
/// - `tasks`：以 ProcId 为键的进程映射表，存储所有进程实体
/// - `ready_queue`：就绪队列，存储等待执行的进程 PID
///
/// 当前使用 FIFO/RR 调度策略。练习题要求改为 stride 调度算法。
pub struct ProcManager {
    /// 所有进程实体的映射表
    tasks: BTreeMap<ProcId, Process>,
    /// 就绪队列（FIFO 调度）
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

/// 实现 Manage trait：进程实体的增删查
impl Manage<Process, ProcId> for ProcManager {
    /// 插入新进程到进程表
    #[inline]
    fn insert(&mut self, id: ProcId, task: Process) {
        self.tasks.insert(id, task);
    }

    /// 根据 PID 获取进程的可变引用
    #[inline]
    fn get_mut(&mut self, id: ProcId) -> Option<&mut Process> {
        self.tasks.get_mut(&id)
    }

    /// 从进程表中删除进程（回收资源）
    #[inline]
    fn delete(&mut self, id: ProcId) {
        self.tasks.remove(&id);
    }
}

/// 实现 Schedule trait：进程调度（当前为 FIFO/RR）
impl Schedule<ProcId> for ProcManager {
    /// 将进程加入就绪队列尾部
    fn add(&mut self, id: ProcId) {
        self.ready_queue.push_back(id);
    }

    /// 从就绪队列头部取出下一个要执行的进程
    fn fetch(&mut self) -> Option<ProcId> {
        self.ready_queue.pop_front()
    }
}
