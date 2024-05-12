//! Process management syscalls

use crate::{
    config::MAX_SYSCALL_NUM,
    loader::get_app_data_by_name,
    mm::{translated_byte_buffer, translated_refmut, translated_str, MapPermission, VirtAddr},
    task::{
        add_task, check_mem_overlap, current_task, current_user_token, delete_vmap,
        exit_current_and_run_next, get_current_task_info, insert_vmap,
        suspend_current_and_run_next, TaskStatus,update_current_task_priority,
    },
    timer::get_time_us,
};
use alloc::sync::Arc;
use core::mem::size_of;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_spawn", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, _path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let new_task = current_task().unwrap().spawn(&data);
        let new_pid = new_task.pid.0;
        // modify trap context of new_task, because it returns immediately after switching
        let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
        // we do not have to move to next instruction since we have done it before
        // for child process, fork returns 0
        trap_cx.x[10] = 0;
        // add new task to scheduler
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let target_ =
        translated_byte_buffer(current_user_token(), ts as *const u8, size_of::<TimeVal>());

    // get time
    let us = get_time_us();
    let tv = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let mut begin = (&tv) as *const _ as *const u8;

    // copy into target
    for target in target_ {
        target.copy_from_slice(unsafe { core::slice::from_raw_parts(begin, target.len()) });
        begin = unsafe { begin.add(target.len()) };
    }
    0
}

/// Finish sys_task_info to pass testcases
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let mut ti = TaskInfo {
        status: TaskStatus::Running,
        syscall_times: [0; MAX_SYSCALL_NUM],
        time: 0,
    };
    if get_current_task_info(&mut ti) == -1 {
        return -1;
    }
    let target_ = translated_byte_buffer(
        current_user_token(),
        _ti as *const u8,
        size_of::<TaskInfo>(),
    );
    let mut begin = (&ti) as *const _ as *const u8;
    for target in target_ {
        target.copy_from_slice(unsafe { core::slice::from_raw_parts(begin, target.len()) });
        begin = unsafe { begin.add(target.len()) };
    }
    0
}

/// mmap: alloc a memory area and map it to the task's virtual memory space
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    let start_va = VirtAddr::from(_start);
    if !start_va.aligned() || _port & !0x7 != 0 || _port & 0x7 == 0 {
        return -1;
    }
    let end_va = VirtAddr::from(_start + _len);
    // check existing
    if check_mem_overlap(start_va, end_va) {
        return -1;
    }
    let mut permission = MapPermission::from_bits((_port as u8) << 1).unwrap();
    permission.set(MapPermission::U, true);
    insert_vmap(start_va, end_va, permission);
    0
}

/// munmap: unmap the memory area
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    let start_va = VirtAddr::from(_start);
    let end_va = VirtAddr::from(_start + _len);
    if delete_vmap(start_va, end_va) {
        return 0;
    }
    -1
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );
    if _prio < 2 {
        return -1;
    }
    update_current_task_priority(_prio as usize);
    _prio
}
