/// Scheduler
pub trait Schedule<I: Copy + Ord> {
    /// 将任务 ID 放入就绪队列。
    fn add(&mut self, id: I);
    /// 从就绪队列取出下一个可运行任务 ID。
    fn fetch(&mut self) -> Option<I>;
}
