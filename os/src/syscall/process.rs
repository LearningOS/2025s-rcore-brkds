//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next,get_task_syscall_counts},
    timer::get_time_us,
};
use super::syscall_map;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    match  _trace_request {
        0 => {
            // syscall trace
            trace!("kernel: syscall trace");
            unsafe {*(_id as * mut u8) as isize }
        }
        1 => {
            // task trace
            trace!("kernel: task trace");
            unsafe {*(_id as * mut u8)= _data as u8;}
            0
        }
        2 => {
            unsafe { (*get_task_syscall_counts())[syscall_map(_id)] as isize}
        }
        _ => {
            panic!("Unsupported trace request!");
        }
        
    }
}
