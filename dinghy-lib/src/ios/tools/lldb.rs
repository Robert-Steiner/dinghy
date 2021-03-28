use std::{
    path::PathBuf,
    process::{Command, Output},
};

use anyhow::{anyhow, Error};

const LLDB: &'static str = "lldb";

pub fn run_source(source: &PathBuf) -> Result<Output, Error> {
    Command::new(LLDB)
        .arg("-s")
        .arg(source)
        .output()
        .map_err(|err| anyhow!("{}", err))
}
