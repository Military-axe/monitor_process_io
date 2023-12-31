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

/// syscall_read_default是默认的read系统调用回调函数
/// 当程序调用read系统调用时，函数会记录read的输入并记录进日志
/// 程序的参数argvs中只需要一个参数，也就是保存的日志地址。
/// 记录read的是读取寄存器的内容
pub fn syscall_read_default(
    regs: user_regs_struct,
    pid: Pid,
    argvs: &Option<Vec<String>>,
) -> PluginStatus {
    // 获取read系统调用的几个参数，由于此阶段已经是系统调用执行结束，
    // 所以rax寄存器记录的是read的字节数也就是返回值
    let syscall_read_addr = regs.rsi;
    let syscall_read_fd = regs.rdi;
    let syscall_read_size = regs.rdx;
    let mut buffer = Vec::new();
    debug!(
        "read_deafult ptrace register => rdx: {:#x}, rsi: {:#x}",
        regs.rdx, regs.rsi
    );

    if is_standard_io(syscall_read_fd) {
        info!("read defaulte read from standard pipe");
        return PluginStatus::StatusOk;
    }

    if unsafe { FIRST_TIME } == true {
        unsafe {
            FIRST_TIME = false;
        }
        return PluginStatus::StatusOk;
    }

    for i in (0..syscall_read_size).step_by(8) {
        match ptrace::read(pid, (syscall_read_addr + i) as AddressType) {
            Ok(read_data) => {
                info!("read from child process {:#x}", read_data);
                if read_data != 0 {
                    // 读到多余的内存就不要了
                    buffer.push(read_data);
                }
            }
            Err(_) => {
                warn!("error read memory from child process");
                return PluginStatus::StatusFailed;
            }
        }
    }
    // 打开文件句柄，之后向此文件读写
    let file_path = match argvs {
        Some(file_path) => file_path.get(0),
        None => {
            warn!("read_default plugin do not config the log_path");
            return PluginStatus::StatusFailed;
        }
    };

    let mut options = OpenOptions::new();
    options.append(true).create(true);
    let mut file_fd = options
        .open(file_path.unwrap())
        .expect("write default could not open file");

    let _ = writeln!(file_fd, "##### read default #####\n");

    // 写入日志
    for i in buffer {
        let s_read = i.to_le_bytes();
        let str_read: Vec<u8> = s_read.iter().copied().filter(|x| *x != 0).collect();
        match file_fd.write(&str_read) {
            Ok(size) => debug!("read_default plugin write success: {}", size),
            Err(_) => {
                error!("read_default plugin could not write in file");
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
