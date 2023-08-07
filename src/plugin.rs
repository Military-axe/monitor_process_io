pub mod read_default;

use crate::conf::PlugConfig;
use log::debug;
use nix::{libc::user_regs_struct, unistd::Pid};
use std::collections::HashMap;

pub enum PluginStatus {
    StatusOk,     // 正常执行
    StatusPass,   // 正常执行，跳过此系统调用后续插件
    StatusError,  // 执行错误，触发panic
    StatusFailed, // 执行失败，继续执行别的插件
}

#[derive(Debug, Clone)]
pub struct PluginInterface {
    pub plug_name: String,
    pub argvs: Option<Vec<String>>,
    pub function: fn(user_regs_struct, Pid, &Option<Vec<String>>) -> PluginStatus,
}

impl PluginInterface {
    pub fn new(
        name: String,
        argvs: Option<Vec<String>>,
        func: fn(user_regs_struct, Pid, &Option<Vec<String>>) -> PluginStatus,
    ) -> Self {
        PluginInterface {
            plug_name: name,
            argvs,
            function: func,
        }
    }
}

/// get_plugininterface_vec函数，传入一个插件配置列表，返回一个Vec<PluginInterface>
/// 记录了所有插件的接口信息
pub fn get_plugininterface_vec(
    plug_conf: Option<Vec<PlugConfig>>,
) -> HashMap<u64, Vec<PluginInterface>> {
    let mut plugin_interface_array = vec![read_default::syscall_read_default];
    let mut hash_table = HashMap::new();
    let config = match plug_conf {
        None => {
            debug!("Do not have plugin config");
            return hash_table;
        }
        Some(config) => config,
    };
    // config中插件的配置顺序，一定需要和plugin_interface_array中的顺序相同
    // TODO: 可以改变PLUGIN_INTERFACE_ARRAY类型为HashMap之类的，通过name来对应配置参数
    for idx in 0..config.len() {
        let syscall_num = config[idx].syscall_number;
        let pl_interface = PluginInterface::new(
            config[idx].name.clone(),
            config[idx].get_argvs(),
            plugin_interface_array.remove(0),
        );
        if hash_table.contains_key(&syscall_num) {
            hash_table.get_mut(&syscall_num).unwrap().push(pl_interface);
        } else {
            hash_table.insert(syscall_num, vec![pl_interface]);
        }
    }
    hash_table
}
