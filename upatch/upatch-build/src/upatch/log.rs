use std::io::LineWriter;
use std::sync::{RwLock, RwLockWriteGuard};
use std::collections::HashMap;
use std::fs::File;

use lazy_static::*;


lazy_static! {
    static ref LOGBUFFER: RwLock<HashMap<String, LineWriter<File>>> = RwLock::new(HashMap::new());
    static ref VERBOSE: RwLock<bool> = RwLock::new(false);
}

pub fn set_log_file(log_file: &str) -> std::io::Result<()> {
    let log_writer =  LineWriter::new(
        std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(log_file)
            .expect("Cannot access log file")
    );
    (*get_log_writer()).insert("log".to_string(), log_writer);
    Ok(())
}


pub fn get_log_writer<'a>() -> RwLockWriteGuard<'a, HashMap<String, LineWriter<File>>> {
    LOGBUFFER.write().expect("get log buffer error")
}

pub fn set_verbose(verbose: bool) -> std::io::Result<()> {
    *VERBOSE.write().unwrap() = verbose;
    Ok(())
}

pub fn verbose(output: &str) {
    match *VERBOSE.read().unwrap() {
        true => println!("{}", output),
        false => ()
    };
}