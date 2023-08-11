use log::{debug, error, info, warn};
use nix::sys::ptrace::{self, AddressType};
use std::fs::OpenOptions;
use std::io::Write;

use crate::plugin::PluginStatus;
use nix::{libc::user_regs_struct, unistd::Pid};

static mut FIRST_TIME: bool = true;

/// 判断是不是从标准流中读取的
fn is_standard_io(reg: u64) -> bool {
    match reg {
        0 => (),
        1 => (),
        2 => (),
        _ => return true,
    }

    false
}

/// syscall_write_default是默认的write系统调用回调函数
/// 当程序调用write系统调用时，函数会记录write的输入并记录进日志
/// 程序的参数argvs中只需要一个参数，也就是保存的日志地址。
/// 记录write的是读取寄存器的内容
pub fn syscall_write_default(
    regs: user_regs_struct,
    pid: Pid,
    argvs: &Option<Vec<String>>,
) -> PluginStatus {
    // 获取write系统调用的几个参数，由于此阶段已经是系统调用执行结束，
    // 所以rax寄存器记录的是write的字节数也就是返回值
    let syscall_write_addr = regs.rsi;
    let syscall_write_fd = regs.rdi;
    let syscall_write_size = regs.rdx;
    let mut buffer = Vec::new();
    debug!(
        "write_deafult ptrace register => rdx: {:#x}, rsi: {:#x}",
        regs.rdx, regs.rsi
    );

    if is_standard_io(syscall_write_fd) {
        info!("write default read from standard pipe");
        return PluginStatus::StatusOk;
    }

    if unsafe { FIRST_TIME } == true {
        unsafe {
            FIRST_TIME = false;
        }
        return PluginStatus::StatusOk;
    }

    for i in (0..syscall_write_size).step_by(8) {
        match ptrace::read(pid, (syscall_write_addr + i) as AddressType) {
            Ok(write_data) => {
                info!("read from child process {:#x}", write_data);
                buffer.push(write_data);
            }
            Err(_) => {
                warn!("error write memory from child process");
                return PluginStatus::StatusFailed;
            }
        }
    }
    // 打开文件句柄，之后向此文件读写
    let file_path = match argvs {
        Some(file_path) => file_path.get(0),
        None => {
            warn!("write_default plugin do not config the log_path");
            return PluginStatus::StatusFailed;
        }
    };
    let mut options = OpenOptions::new();
    options.append(true).create(true);
    let mut file_fd = options
        .open(file_path.unwrap())
        .expect("write default could not open file");

    // 写入日志

    let _ = writeln!(file_fd, "##### write default #####\n");

    for i in buffer {
        let s_write = i.to_le_bytes();
        let str_write: Vec<u8> = s_write.iter().copied().filter(|x| *x != 0).collect();
        match file_fd.write(&str_write) {
            Ok(size) => debug!("write_default plugin write success: {}", size),
            Err(_) => {
                error!("write_default plugin could not write in file");
                return PluginStatus::StatusError;
            }
        }
    }

    let _ = file_fd.write(&[0xa]);

    unsafe {
        FIRST_TIME = true;
    }

    PluginStatus::StatusOk
}
