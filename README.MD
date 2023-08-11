# monitor process io

设计初衷是针对AWD中pwn题，怎么捕获AWD pwn题的交互流量。

参考[pwn_waf](https://github.com/cnitlrt/pwn_waf)的设计原理，原理是将原本的pwn程序当作子进程，父进程通过ptrace来监控子进程的syscall，当子进程syscall时捕获信号并判断是否是read/write系统调用，如果是，则读取子进程对应寄存器，然后进一步读取对应内存

```
origin process:

									+-----+
	nc ipaddress port	-------->	| pwn | 
									+-----+

new:
									+---------+
	nc ipaddress port	-------->	| parrent | ----------- ptrace -----------+
									+---------+                               |
									     |                                    |
										 |      +-------+                  +-----+
										 +----> | child | ---- execvp ---- | pwn |
										        +-------+                  +-----+
```

本项目使用rust语言编写，通过config.toml文件来配置，同时加入了插件体系。

## 配置文件

配置文件的默认路径是`conf/config.toml`,文件格式中有两种，1是项目配置，2是插件配置

### 项目配置

config关键字是key，然后下面有3个key

- file_path: 想要监控io的程序路径（比如pwn题的附件）
- argv: 这个需要监控程序的命令函参数，没有命令行参数的话此项任意填即可
- arch: 此项目的架构，目前只区分64/32（TODO: 未开发32）

```toml
[config]

file_path = '/mnt/d/Documents/git_down/monitor_process_io/test/main'
argv = '12345'
arch = '64'
```

### 插件体系

插件体系以列表形式加入toml，列表名为`plugin`,下面有参数设置

- syscall_number: 需要监控的系统调用号
- name: 插件名称
- argv_num: 插件参数（根据不同插件不同配置）
- argv1: 第一个插件参数[可选]
- argv2: 第二个插件参数[可选]
- argv3: 第三个插件参数[可选]
- argv4: 第四个插件参数[可选]

目前最多设置4个值给到插件，比如默认的read_defailt和write_default插件是记录子进程read/write系统调用并存储到文件中，文件路径是插件的`argv1`的值，如下

```toml
[[plugin]]

syscall_number = 0
name = 'read_default'
argv_num = 1
argv1 = 'plugin_default.log'

[[plugin]]

syscall_number = 1
name = 'write_default'
argv_num = 1
argv1 = 'plugin_default.log'
```

## 自定义插件

可以根据需求添加自定义插件到源码中，编译后即可加入

### 插件接口

插件接口如下

```rust
#[derive(Debug, Clone)]
pub struct PluginInterface {
    pub plug_name: String,
    pub argvs: Option<Vec<String>>,
    pub function: fn(user_regs_struct, Pid, &Option<Vec<String>>) -> PluginStatus,
}
```

自定义插件需要有一个函数形式如同`PluginInterface::function`所示，3个参数，子进程寄存器结构体，子进程`Pid`结构体，以及插件参数列表`&Option<Vec<String>>`

1. 将自定义插件`custom.rs`加入到`src/plugin`目录下
2. 在`src/plugin.rs`中的`get_plugininterface_vec`函数下的`plugin_interface_array`列表中加入自定义函数指针，记得要在plugin.rs导入自定义模块

添加函数指针

```rust
pub fn get_plugininterface_vec(
    plug_conf: Option<Vec<PlugConfig>>,
) -> HashMap<u64, Vec<PluginInterface>> {
    let mut plugin_interface_array = vec![
        read_default::syscall_read_default,
        write_default::syscall_write_default,
    ];
    ...
}
```

导入文件

```rust
pub mod read_default;
pub mod write_default;
```
3. 在`conf/config.toml`添加插件配置，添加顺序必须于`plugin_interface_array`列表中的函数指针顺序相同

## 编译

可以直接cargo来编译，但是这样编译的不是静态编译。

建议使用MUSL来静态编译

[MUSL 支持完全静态二进制文件 - Rust 版本指南 中文版 (rustwiki.org)](https://rustwiki.org/zh-CN/edition-guide/rust-2018/platform-and-target-support/musl-support-for-fully-static-binaries.html)

[rust 静态编译可执行文件指南_castellan的博客-CSDN博客](https://blog.csdn.net/castellan/article/details/86063775)