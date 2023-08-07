pub mod conf;
pub mod plugin;

use conf::Config;
use log::debug;
use nix::libc::user_regs_struct;
use nix::sys::ptrace;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{fork, ForkResult, Pid};
use plugin::*;
use std::collections::HashMap;
use std::{os::unix::process::CommandExt, process::Command};

/// 临时的log函数，用来代替单纯的println!，之后可以在log中做拓展
///
/// # Example
///
/// ```rust
/// let m = String::From("12345");
/// log(m)
/// ```
fn log(s: String) {
    println!("{}", s);
}

/// 运行插件列表中的插件，并处理插件返回的状态
/// 插件需要保证能正常返回状态并处理异常，外部状态不再处理异常
fn run_plugins(pl: &Vec<PluginInterface>, user_regs: user_regs_struct, child_pid: Pid) {
    for idx in pl {
        debug!("run plugin: {}", idx.plug_name);
        let status = (idx.function)(user_regs, child_pid, &idx.argvs);
        match status {
            PluginStatus::StatusOk => (),
            PluginStatus::StatusPass => break,
            PluginStatus::StatusError => panic!("Error run the Plugin: {}", idx.plug_name),
            PluginStatus::StatusFailed => log(format!("Failed to run plugin {}", idx.plug_name)),
        }
    }
}

fn child_process(config: &Config) {
    // child process to run the object
    let status;

    status = ptrace::traceme();
    match status {
        Ok(_) => (),
        Err(err) => panic!("Failed to ptrace parent in child process {:?}", err),
    }

    Command::new(config.get_file())
        .arg(config.get_argv())
        .exec();
}

/// 父进程负责循环等待子进程发送的信号，接受到信号后读取系统调用寄存器（在x64中是rax）
/// 对比寄存器中的值，如果值是我们的目标系统调用，就执行对应系统调用的记录操作
fn parent_process(child: Pid, table: HashMap<u64, Vec<PluginInterface>>) {
    // parent process

    loop {
        match waitpid(child, None) {
            Ok(WaitStatus::Stopped(_, _)) => (),
            Ok(WaitStatus::Exited(_, _)) => break,
            Ok(_) => continue,
            Err(err) => panic!("Failed to wait for process: {:?}", err),
        };

        let regs = match ptrace::getregs(child) {
            Ok(regs) => regs,
            Err(err) => panic!("Failed to get registers: {:?}", err),
        };

        // 判断系统调用号是插件对应的目标系统调用号
        // 如果此系统调用是插件对应的系统调用，则循环调用回调函数列表
        match table.get(&regs.orig_rax) {
            Some(&ref pi_list) => run_plugins(pi_list, regs, child),
            None => {
                debug!("Syscall number {} not in HashMap", regs.orig_rax);
                match ptrace::syscall(child, None) {
                    Ok(_) => continue,
                    Err(err) => panic!("Failed to execute syscall: {:?}", err),
                };
            }
        }

        // 向PID进程发送PTRACE_SYSCALL信号，当在PID进程下一次系统调用时，会向当前进程发送
        // 信号并返回系统调用结果
        match ptrace::syscall(child, None) {
            Ok(_) => (),
            Err(err) => panic!("Failed to execute syscall: {:?}", err),
        };
    }
}

#[cfg(test)]
#[test]
fn test() {
    let c = conf::read_config(&String::from(
        "/mnt/d/Documents/git_down/monitor_process_io/monitor_process_io/conf/config.toml",
    ));
    println!("{:?}", c);
}

fn main() {
    env_logger::init();
    let pid;
    let config = conf::read_config(&String::from(
        "/mnt/d/Documents/git_down/monitor_process_io/monitor_process_io/conf/config.toml",
    ));
    let plugin_interface_list = get_plugininterface_vec(config.plugin);

    unsafe {
        pid = fork();
    }
    debug!("fock success");
    match pid {
        Ok(ForkResult::Parent { child }) => parent_process(child, plugin_interface_list),
        Ok(ForkResult::Child) => child_process(&config.config),
        Err(_) => println!("Fork failed"),
    }

    // thread::sleep(time::Duration::from_millis(10000));
}
