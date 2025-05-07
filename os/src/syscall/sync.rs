use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
        let mut process_inner = process.inner_exclusive_access();
        println!("mutex pid{} tid{} {}  {}-------------------------------------------------------",process.pid.0,tid,process_inner.mutex_available.len(),process_inner.mutex_list.len());

    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {   while process_inner.mutex_available.len() < id + 1 {
            process_inner.mutex_available.push(0);
        }
        //println!("mutex id {} {} {}-------------------------------------------------------",id,process_inner.mutex_available.len(),process_inner.mutex_list.len());
        process_inner.mutex_available[id] = 1;
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        while process_inner.mutex_available.len() < process_inner.mutex_list.len() {
            process_inner.mutex_available.push(0);
        }
        process_inner.mutex_available.push(1);
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    while  process_inner.mutex_matrix[1][tid].len() < mutex_id +1{
        process_inner.mutex_matrix[1][tid].push(0);
    }
    while  process_inner.mutex_matrix[0][tid].len() < mutex_id +1{
        process_inner.mutex_matrix[0][tid].push(0);
    }
    process_inner.mutex_matrix[1][tid][mutex_id]+=1;

    if process_inner.deadlock_detect &&process_inner.deadlock_detect().0{
        process_inner.mutex_matrix[1][tid][mutex_id] -= 1;
        -0xDEAD
    }
    else{
        drop(process_inner);
        drop(process);
        mutex.lock();
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.mutex_available[mutex_id] -= 1;
        process_inner.mutex_matrix[1][tid][mutex_id]-=1;
        process_inner.mutex_matrix[0][tid][mutex_id]+=1;
        0
    }
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    process_inner.mutex_available[mutex_id] += 1;
    process_inner.mutex_matrix[0][tid][mutex_id]-=1;
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    println!("semaphore id{}{}  {} {}-------------------------------------------------------",process.pid.0,tid,process_inner.semaphore_available.len(),process_inner.semaphore_list.len());
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {   
        // println!("semaphore id {} {} {}-------------------------------------------------------",id,process_inner.semaphore_available.len(),process_inner.semaphore_list.len());
        while  process_inner.semaphore_available.len() <id +1
        {process_inner.semaphore_available.push(0);};
        process_inner.semaphore_available[id] = res_count as i32;
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        while  process_inner.semaphore_available.len() <process_inner.semaphore_list.len()
        {process_inner.semaphore_available.push(0);};
        process_inner.semaphore_available.push(res_count as i32);
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    process_inner.semaphore_available[sem_id] += 1;
    drop(process_inner);
    sem.up();
    0
    
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    while process_inner.semaphore_matrix[1][tid].len() < sem_id + 1 {
        process_inner.semaphore_matrix[1][tid].push(0);
    }
    while process_inner.semaphore_matrix[0][tid].len() < sem_id + 1 {
        process_inner.semaphore_matrix[0][tid].push(0);
    }
    process_inner.semaphore_matrix[1][tid][sem_id] += 1;
    if process_inner.deadlock_detect &&process_inner.deadlock_detect().1{
        process_inner.semaphore_matrix[1][tid][sem_id] -= 1;
         -0xDEAD
    }
    else{
        drop(process_inner);
        sem.down();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.semaphore_available[sem_id] -= 1;
        process_inner.semaphore_matrix[1][tid][sem_id] -= 1;
        process_inner.semaphore_matrix[0][tid][sem_id] += 1;
        0
    }
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect NOT IMPLEMENTED");
    if _enabled != 0&& _enabled != 1 {
        return -1
    } else {
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.deadlock_detect = if _enabled == 1 {
            true
        } else {
            false
        };
        0
    }
}
