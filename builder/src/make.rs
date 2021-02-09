// use std::collections::HashMap;
use std::process::Child;
use std::process::Command;
use std::fs::File;
use std::vec::Vec;

#[allow(dead_code)]
pub struct Make {
    // configs: HashMap<&'a str, &'a str>,
    configs: Vec<String>,
    log_file: Option<File>,
}


impl Make {
    pub fn new() -> Self {
        Self { 
            configs : Vec::new(),
            log_file: None,
         }
    }


    pub fn with_flag(mut self, flag: &str, value: &str) -> Make {
        self.configs.push(flag.to_string());
        self.configs.push(value.to_string());
        self
    }


    pub fn spawn(&self) -> std::io::Result<Child> {
        Command::new("/usr/bin/make")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .args(&self.configs)
            .spawn()
    }
}