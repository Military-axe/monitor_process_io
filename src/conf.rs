use serde::Deserialize;
use std::{fs::File, io::Read};
use toml;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    file_path: Option<String>,
    argv: Option<String>,
    pub arch: Option<String>,
}

impl Config {
    pub fn get_file(&self) -> String {
        match &self.file_path {
            Some(file) => file.clone(),
            None => panic!("Don't get the file path"),
        }
    }

    pub fn get_argv(&self) -> String {
        match &self.argv {
            Some(argv) => argv.clone(),
            None => panic!("Don't get the file path"),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PlugConfig {
    pub name: String,
    pub syscall_number: u64,
    argv_num: Option<u8>,
    argv1: Option<String>,
    argv2: Option<String>,
    argv3: Option<String>,
    argv4: Option<String>,
}

impl PlugConfig {
    pub fn get_argvs(&self) -> Option<Vec<String>> {
        let argv_num = match self.argv_num {
            Some(argv_num) => argv_num,
            None => return None,
        };

        if argv_num == 0 {
            return None;
        }

        let mut r = Vec::new();

        if argv_num >= 1 {
            r.push(self.argv1.clone().unwrap())
        }

        if argv_num >= 2 {
            r.push(self.argv2.clone().unwrap())
        }

        if argv_num >= 3 {
            r.push(self.argv3.clone().unwrap())
        }

        if argv_num >= 4 {
            r.push(self.argv4.clone().unwrap())
        }

        return Some(r);
    }
}

#[derive(Debug, Deserialize)]
pub struct Conf {
    pub config: Config,
    pub plugin: Option<Vec<PlugConfig>>
}

/// 读取配置文件，配置文件以toml文件格式存储，其中中属性应该是如果Config结构体，
/// 4个属性具体内容如下，都是以字符串形式存储
/// file_path -- 可执行文件路径，也就是原本需要执行文件的路劲
/// argv -- 可执行文件的命令行执行参数
/// arch -- 原本程序的架构，64/32
/// log_path -- 记录程序io的文件路径
///
/// # Example
///
/// ```rust
/// let c = conf::read_config(&String::from("/mnt/d/Documents/git_down/monitor_process_io/monitor_process_io/conf/config.toml"));
/// println!("{:?}", c);
/// ```
/// 结果
/// ```
/// Config { file_path: Some("123"), argv: Some("12345"), arch: Some("64"), log_path: Some("xxx") }
/// ```
pub fn read_config(config_path: &String) -> Conf {
    let mut file = match File::open(config_path) {
        Ok(f) => f,
        Err(e) => panic!("no such file {} exception:{}", config_path, e),
    };

    let mut str_val = String::new();

    match file.read_to_string(&mut str_val) {
        Ok(s) => s,
        Err(e) => panic!("Error Reading file: {}", e),
    };
    let config: Conf = toml::from_str(&str_val).unwrap();

    return config;
}
