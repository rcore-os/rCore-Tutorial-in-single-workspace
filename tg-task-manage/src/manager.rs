/// Manager trait
pub trait Manage<T, I: Copy + Ord> {
    /// 插入一个任务对象（或进程/线程实体）。
    fn insert(&mut self, id: I, item: T);
    /// 删除指定 ID 的任务对象。
    fn delete(&mut self, id: I);
    /// 获取指定 ID 的可变引用（用于更新运行时状态）。
    fn get_mut(&mut self, id: I) -> Option<&mut T>;
}
