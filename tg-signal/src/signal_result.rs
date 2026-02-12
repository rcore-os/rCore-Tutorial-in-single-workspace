/// 信号处理函数返回得到的结果
pub enum SignalResult {
    /// 没有信号需要处理
    NoSignal,
    /// 目前正在处理信号，因而无法接受其他信号
    IsHandlingSignal,
    /// 已经处理了一个信号，接下来正常返回用户态即可
    Ignored,
    /// 已经处理了一个信号，并修改了用户上下文
    Handled,
    /// 需要结束当前进程，并给出退出时向父进程返回的 errno
    ProcessKilled(i32),
    /// 需要暂停当前进程，直到其他进程给出继续执行的信号
    ProcessSuspended,
}

// 教程提示：
// `SignalResult` 本质上是“trap 返回前调度决策”的枚举化结果，
// 调用方可据此决定：直接回用户态、切换任务、或结束当前进程。
