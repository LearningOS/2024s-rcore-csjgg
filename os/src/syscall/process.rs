//! Process management syscalls
use core::mem::size_of;

use crate::{
    config::MAX_SYSCALL_NUM,
    mm::{translated_byte_buffer, MapPermission, VirtAddr},
    task::{
        change_program_brk, check_mem_overlap, current_user_token,
        exit_current_and_run_next, get_current_task_info, suspend_current_and_run_next, TaskStatus,
        insert_vmap,delete_vmap,
    },
    timer::get_time_us,
};

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

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
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
    if delete_vmap(start_va, end_va){
        return 0;
    }
    -1
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
