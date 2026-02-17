#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exit, fork, getpid, sched_yield, sleep, wait};

const DEPTH: usize = 4;

// 教学目标：
// 以二叉树方式递归 fork，验证进程层级关系与 wait 回收顺序。

fn fork_child(cur: &str, branch: char) {
    let mut next = [0u8; DEPTH + 1];
    let l = cur.len();
    if l >= DEPTH {
        return;
    }
    next[..l].copy_from_slice(cur.as_bytes());
    next[l] = branch as u8;
    if fork() == 0 {
        // 子进程继续递归扩展自己的子树。
        fork_tree(core::str::from_utf8(&next[..l + 1]).unwrap());
        sched_yield();
        exit(0);
    }
}

fn fork_tree(cur: &str) {
    println!("pid{}: {}", getpid(), cur);
    fork_child(cur, '0');
    fork_child(cur, '1');
    let mut exit_code: i32 = 0;
    for _ in 0..2 {
        // 每个内部节点等待其两个孩子结束。
        wait(&mut exit_code);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    fork_tree("");
    let mut exit_code: i32 = 0;
    for _ in 0..2 {
        wait(&mut exit_code);
    }
    sleep(3000);
    0
}
