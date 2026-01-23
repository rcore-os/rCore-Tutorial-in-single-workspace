#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{get_time, set_priority};

fn spin_delay() {
    let mut j = true;
    for _ in 0..10 {
        j = !j;
    }
}

const MAX_TIME: isize = 4000;

fn count_during(prio: isize) -> isize {
    let start_time = get_time();
    let mut acc = 0;
    set_priority(prio);
    loop {
        spin_delay();
        acc += 1;
        if acc % 400 == 0 {
            let time = get_time() - start_time;
            if time > MAX_TIME {
                return acc;
            }
        }
    }
}

#[no_mangle]
extern "C" fn main() -> i32 {
    let prio = 6;
    let count = count_during(prio);
    println!(
        "priority = {}, exitcode = {}, ratio = {}",
        prio,
        count,
        count / prio
    );
    0
}
