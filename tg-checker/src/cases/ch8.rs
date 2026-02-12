//! Chapter 8 test cases

use super::TestCase;

/// ch8 base test
pub fn base() -> TestCase {
    // ch8 base 重点覆盖线程/同步原语（mutex/semaphore/condvar）基础功能。
    TestCase {
        expected: vec![
            // inherited from ch6b (without sbrk)
            "Hello, world from user mode program!",
            "Test power_3 OK!",
            "Test power_5 OK!",
            "Test power_7 OK!",
            "Test write A OK!",
            "Test write B OK!",
            "Test write C OK!",
            "exit pass.",
            "hello child process!",
            r"child process pid = (\d+), exit code = (\d+)",
            "forktest pass.",
            "file_test passed!",
            // ch7b_pipetest
            "pipetest passed!",
            // ch8b_mpsc_sem
            "mpsc_sem passed!",
            // ch8b_phil_din_mutex
            "philosopher dining problem with mutex test passed!",
            // ch8b_race_adder_mutex_spin
            "race adder using spin mutex test passed!",
            // ch8b_sync_sem
            "sync_sem passed!",
            // ch8b_test_condvar
            "test_condvar passed!",
            // ch8b_threads_arg
            "threads with arg test passed!",
            // ch8b_threads
            "threads test passed!",
        ],
        not_expected: vec!["FAIL: T.T", "Test sbrk failed!"],
    }
}

/// ch8 exercise test
pub fn exercise() -> TestCase {
    // ch8 exercise 增加死锁相关实验输出检查。
    TestCase {
        expected: vec![
            // inherited from ch6b (without sbrk)
            "Hello, world from user mode program!",
            "Test power_3 OK!",
            "Test power_5 OK!",
            "Test power_7 OK!",
            "Test write A OK!",
            "Test write B OK!",
            "Test write C OK!",
            "exit pass.",
            "hello child process!",
            r"child process pid = (\d+), exit code = (\d+)",
            "forktest pass.",
            "file_test passed!",
            // ch7b_pipetest
            "pipetest passed!",
            // ch8b_mpsc_sem
            "mpsc_sem passed!",
            // ch8b_phil_din_mutex
            "philosopher dining problem with mutex test passed!",
            // ch8b_race_adder_mutex_spin
            "race adder using spin mutex test passed!",
            // ch8b_sync_sem
            "sync_sem passed!",
            // ch8b_test_condvar
            "test_condvar passed!",
            // ch8b_threads_arg
            "threads with arg test passed!",
            // ch8b_threads
            "threads test passed!",
            // ch8_deadlock_mutex1
            "deadlock test mutex 1 OK!",
            // ch8_deadlock_sem1
            "deadlock test semaphore 1 OK!",
            // ch8_deadlock_sem2
            "deadlock test semaphore 2 OK!",
        ],
        not_expected: vec!["FAIL: T.T", "Test sbrk failed!"],
    }
}
