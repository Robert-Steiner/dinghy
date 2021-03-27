use std::{
    path::PathBuf,
    process::{Command, Output},
};

use anyhow::{anyhow, Error};

pub fn run_source(source: &PathBuf) -> Result<Output, Error> {
    Command::new("lldb")
        .arg("-s")
        .arg(source)
        .output()
        .map_err(|err| anyhow!("{}", err))
}
