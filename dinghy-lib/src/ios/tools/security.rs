use std::{
    path::PathBuf,
    process::{Command, Output},
};

use anyhow::{anyhow, Error};

const SECURITY: &'static str = "security";

pub fn find_identities() -> Result<Output, Error> {
    Command::new(SECURITY)
        .args(&["find-identity", "-v", "-p", "codesigning"])
        .output()
        .map_err(|err| anyhow!("{}", err))
}

pub fn find_certificate(name: &str) -> Result<Output, Error> {
    Command::new(SECURITY)
        .args(&["find-certificate", "-a", "-c", name, "-p"])
        .output()
        .map_err(|err| anyhow!("{}", err))
}

pub fn decode_cms(file: &PathBuf) -> Result<Output, Error> {
    Command::new(SECURITY)
        .arg("cms")
        .arg("-D")
        .arg("-i")
        .arg(file)
        .output()
        .map_err(|err| anyhow!("{}", err))
}
