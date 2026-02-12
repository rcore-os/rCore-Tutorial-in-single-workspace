use super::{SignalNo, SignalResult};

/// 没有处理函数时的默认行为。
///
/// 参见 `https://venam.nixers.net/blog/unix/2016/10/21/unix-signals.html`
pub(crate) enum DefaultAction {
    /// 结束进程。其实更标准的实现应该细分为 terminate / terminate(core dump) / stop
    Terminate(i32),
    /// 忽略信号
    Ignore,
}

impl From<SignalNo> for DefaultAction {
    fn from(signal_no: SignalNo) -> Self {
        // 教学实现中仅保留“忽略/终止”两种核心行为，便于实验聚焦主流程。
        match signal_no {
            SignalNo::SIGCHLD | SignalNo::SIGURG => Self::Ignore,
            _ => Self::Terminate(-(signal_no as i32)),
        }
    }
}

impl Into<SignalResult> for DefaultAction {
    fn into(self) -> SignalResult {
        // 将“动作语义”转换为内核调度层可消费的执行结果。
        match self {
            Self::Terminate(exit_code) => SignalResult::ProcessKilled(exit_code),
            Self::Ignore => SignalResult::Ignored,
        }
    }
}
