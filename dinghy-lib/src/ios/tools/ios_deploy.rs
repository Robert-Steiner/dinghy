use std::{path::PathBuf, process::Command};

use anyhow::Error;

use crate::ios::command_ext::ExitStatusExt;

pub fn launch_app(args: &[&str], envs: &[&str], bundle_dir: &PathBuf) -> Result<(), Error> {
    Command::new("ios-deploy")
        .args(&["--args", &args.join(" ")])
        .args(&["--envs", &envs.join(" ")])
        .args(&["--noninteractive", " --debug", "--bundle"])
        .arg(bundle_dir)
        .status()?
        .expect_success()
}
