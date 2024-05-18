use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task, TaskControlBlock};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
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
    let flag;
    // check open detect
    {
        let process = current_process();
        let process_inner = process.inner_exclusive_access();
        flag = process_inner.open_detect;
    }
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        if flag{
            // add to mutex_res
            process_inner.mutex_res.push(1);
            // add to all task
            for task in process_inner.tasks.iter_mut() {
                if let Some(task) = task {
                    task.inner_exclusive_access().mutex_alloc.push(0);
                    task.inner_exclusive_access().mutex_need.push(0);
                }
            }
        }
        process_inner.mutex_list.len() as isize - 1
    }
}

/// find task whose mutex_need <= work and finish is false
fn find_task_mutex(
    tasks: &Vec<Option<Arc<TaskControlBlock>>>,
    work: &Vec<i32>,
    finish: &Vec<bool>,
) -> Option<usize> {
    for (i, task) in tasks.iter().enumerate() {
        if let Some(task) = task {
            let task_inner = task.inner_exclusive_access();
            if finish[i] == false
                && task_inner
                    .mutex_need
                    .iter()
                    .zip(work.iter())
                    .all(|(need, work)| need <= work)
            {
                return Some(i);
            }
        }
    }
    None
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
    let flag;
    // check open detect
    {
        let process = current_process();
        let process_inner = process.inner_exclusive_access();
        flag = process_inner.open_detect;
    }
    if flag{
        // add current task's mutex_need 1
        {
            let current_task = current_task().unwrap();
            let mut current_task_inner = current_task.inner_exclusive_access();
            current_task_inner.mutex_need[mutex_id] += 1;
        }
    }
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    if flag{
        // check all tasks
        let mut finish = vec![false; process_inner.tasks.len()];
        let mut work = process_inner.mutex_res.clone();
        let tasks = &process_inner.tasks;
        loop {
            let task_id = find_task_mutex(tasks, &work, &finish);
            if let Some(task_id) = task_id {
                // add to work and mark finish as true
                let task = tasks[task_id].as_ref().unwrap();
                let task_inner = task.inner_exclusive_access();
                for (a, b) in work.iter_mut().zip(task_inner.mutex_alloc.iter()) {
                    *a += b;
                }
                finish[task_id] = true;
            } else {
                break;
            }
        }
        // if finish has false, return -1
        if finish.iter().any(|&x| x == false) {
            return -0xDEAD;
        }
    }

    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    if flag{
        // del current task's mutex_need 1 and add mutex_alloc 1
        {
            let current_task = current_task().unwrap();
            let mut current_task_inner = current_task.inner_exclusive_access();
            current_task_inner.mutex_need[mutex_id] -= 1;
            current_task_inner.mutex_alloc[mutex_id] += 1;
        }
        // del process mutex_res 1
        {
            let process = current_process();
            let mut process_inner = process.inner_exclusive_access();
            process_inner.mutex_res[mutex_id] -= 1;
        }
    }
    0
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
    let flag;
    // check open detect
    {
        let process = current_process();
        let process_inner = process.inner_exclusive_access();
        flag = process_inner.open_detect;
    }
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    if flag{
        // add process mutex_res 1
        {
            let process = current_process();
            let mut process_inner = process.inner_exclusive_access();
            process_inner.mutex_res[mutex_id] += 1;
        }
        // del current task's mutex_alloc 1
        {
            let current_task = current_task().unwrap();
            let mut current_task_inner = current_task.inner_exclusive_access();
            current_task_inner.mutex_alloc[mutex_id] -= 1;
        }
    }
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
    let flag;
    // check open detect
    {
        let process = current_process();
        let process_inner = process.inner_exclusive_access();
        flag = process_inner.open_detect;
    }
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        if flag{
            // add to semaphore_res
            process_inner.semaphore_res.push(res_count as i32);
            // add to all task
            for task in process_inner.tasks.iter_mut() {
                if let Some(task) = task {
                    task.inner_exclusive_access().semaphore_alloc.push(0);
                    task.inner_exclusive_access().semaphore_need.push(0);
                }
            }
        }
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
    let flag;
    // check open detect
    {
        let process = current_process();
        let process_inner = process.inner_exclusive_access();
        flag = process_inner.open_detect;
    }
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    if flag{
        // add process semaphore_res 1
        {
            let process = current_process();
            let mut process_inner = process.inner_exclusive_access();
            process_inner.semaphore_res[sem_id] += 1;
        }
        // del current task's semaphore_alloc 1
        {
            let current_task = current_task().unwrap();
            let mut current_task_inner = current_task.inner_exclusive_access();
            current_task_inner.semaphore_alloc[sem_id] -= 1;
        }
    }
    0
}

/// find task whose semaphore_need <= work and finish is false
fn find_task_semaphore(
    tasks: &Vec<Option<Arc<TaskControlBlock>>>,
    work: &Vec<i32>,
    finish: &Vec<bool>,
) -> Option<usize> {
    for (i, task) in tasks.iter().enumerate() {
        if let Some(task) = task {
            let task_inner = task.inner_exclusive_access();
            if finish[i] == false
                && task_inner
                    .semaphore_need
                    .iter()
                    .zip(work.iter())
                    .all(|(need, work)| need <= work)
            {
                return Some(i);
            }
        }
    }
    None
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
    let flag;
    // check open detect
    {
        let process = current_process();
        let process_inner = process.inner_exclusive_access();
        flag = process_inner.open_detect;
    }
    if flag{
        // add current task's semaphore_need 1
        {
            let current_task = current_task().unwrap();
            let mut current_task_inner = current_task.inner_exclusive_access();
            current_task_inner.semaphore_need[sem_id] += 1;
        }
    }
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    if flag{

        // check all tasks
        let mut finish = vec![false; process_inner.tasks.len()];
        let mut work = process_inner.semaphore_res.clone();
        let tasks = &process_inner.tasks;
        loop {
            let task_id = find_task_semaphore(tasks, &work, &finish);
            if let Some(task_id) = task_id {
                // add to work and mark finish as true
                let task = tasks[task_id].as_ref().unwrap();
                let task_inner = task.inner_exclusive_access();
                for (a, b) in work.iter_mut().zip(task_inner.semaphore_alloc.iter()) {
                    *a += b;
                }
                finish[task_id] = true;
            } else {
                break;
            }
        }
        // if finish has false, return -1
        if finish.iter().any(|&x| x == false) {
            return -0xDEAD;
        }
    }
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.down();
    if flag{
        // del current task's semaphore_need 1 and add semaphore_alloc 1
        {
            let current_task = current_task().unwrap();
            let mut current_task_inner = current_task.inner_exclusive_access();
            current_task_inner.semaphore_need[sem_id] -= 1;
            current_task_inner.semaphore_alloc[sem_id] += 1;
        }
        // del process semaphore_res 1
        {
            let process = current_process();
            let mut process_inner = process.inner_exclusive_access();
            process_inner.semaphore_res[sem_id] -= 1;
        }
    }
    0
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
    trace!("kernel: sys_enable_deadlock_detect");
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if _enabled == 1{
        process_inner.open_detect = true;
        0
    }else if _enabled == 0{
        process_inner.open_detect = false;
        0
    }else{
        -1
    }
}
