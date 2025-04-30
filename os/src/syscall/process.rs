//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next,current_task_id,current_memory_set};
use crate::mm::{VirtAddr,PTEFlags,frame_alloc,StepByOne};
use crate::config::PAGE_SIZE;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    -1
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    -1
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    print!("mmap\n");
    let pt = current_memory_set();
    println!("pt is {:?}, current_task_id is {} ,_port is {}\n",unsafe{(*pt).token()},current_task_id(),_port);
    let mut start = VirtAddr(_start).floor();
    if _len == 0|| _port==0 ||  VirtAddr(_start).page_offset()!=0 || _port>7{
        return -1;
    }
    let end = (_start+_len-1)/PAGE_SIZE+1;
    let len:usize = end-_start/PAGE_SIZE;
    for _i in 0..len {
        let ft = frame_alloc().unwrap();
        let ppn = ft.ppn;
        let re = unsafe{(*pt).map(start, ppn, PTEFlags::from_bits((_port<<1) as u8 ).unwrap()|PTEFlags::U)};
        if re == 0 {
            return -1;
        }
        println!("start is {:?}, ft is {:?}",start,ft);
        start.step();
    }
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    print!("munmap\n");
    let pt = current_memory_set();
    println!("pt is {:?}, current_task_id is {}\n",unsafe{(*pt).token()},current_task_id());
    let mut start = VirtAddr(_start).floor();
    if _len == 0 ||  VirtAddr(_start).page_offset()!=0 {
        return -1;
    }
    let end = (_start+_len-1)/PAGE_SIZE+1;
    let len:usize = end-_start/PAGE_SIZE;
    for _i in 0..len {
        let re = unsafe{(*pt).unmap(start)};
        if re == 0 {
            return -1;
        }
        println!("start is {:?}",start);
        start.step();
    }
    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
