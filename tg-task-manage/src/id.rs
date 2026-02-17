use core::sync::atomic::{AtomicUsize, Ordering};

/// 进程 Id
#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash, Ord, PartialOrd)]
pub struct ProcId(usize);

impl ProcId {
    /// 分配一个新的进程 ID（单调递增，不复用）。
    pub fn new() -> Self {
        // 任务编号计数器，任务编号自增
        static PID_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = PID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }
    /// 从原始整数构造进程 ID（常用于特殊值或反序列化）。
    pub fn from_usize(v: usize) -> Self {
        Self(v)
    }
    /// 取出底层整数值。
    pub fn get_usize(&self) -> usize {
        self.0
    }
}

/// 线程 Id
#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash, Ord, PartialOrd)]
pub struct ThreadId(usize);

impl ThreadId {
    /// 分配一个新的线程 ID（单调递增，不复用）。
    pub fn new() -> Self {
        // 任务编号计数器，任务编号自增
        static TID_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = TID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }
    /// 从原始整数构造线程 ID。
    pub fn from_usize(v: usize) -> Self {
        Self(v)
    }
    /// 取出底层整数值。
    pub fn get_usize(&self) -> usize {
        self.0
    }
}

/// 协程 Id
#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash, Ord, PartialOrd)]
pub struct CoroId(usize);

impl CoroId {
    /// 分配一个新的协程 ID（单调递增，不复用）。
    pub fn new() -> Self {
        // 任务编号计数器，任务编号自增
        static CID_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = CID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }
    /// 从原始整数构造协程 ID。
    pub fn from_usize(v: usize) -> Self {
        Self(v)
    }
    /// 取出底层整数值。
    pub fn get_usize(&self) -> usize {
        self.0
    }
}
