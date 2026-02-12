#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

const LEN: usize = 100;

// 教学目标：
// 长时间计算 + 周期输出，验证用户程序执行稳定性（参数为 3 的版本）。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    let p = 3u64;
    let m = 998244353u64;
    let iter: usize = 200000;
    let mut s = [0u64; LEN];
    let mut cur = 0usize;
    s[cur] = 1;
    for i in 1..=iter {
        let next = if cur + 1 == LEN { 0 } else { cur + 1 };
        s[next] = s[cur] * p % m;
        cur = next;
        if i % 10000 == 0 {
            println!("power_3 [{}/{}]", i, iter);
        }
    }
    println!("{}^{} = {}(MOD {})", p, iter, s[cur], m);
    println!("Test power_3 OK!");
    0
}
