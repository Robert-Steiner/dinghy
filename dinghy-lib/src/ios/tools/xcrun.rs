use std::process::Command;

use anyhow::{anyhow, Error};
use log::debug;
use simctl::{list::DeviceState, Device, DeviceQuery, Simctl};

const XCRUN: &'static str = "xcrun";

pub fn launch_app(dev_id: &str, app_id: &str, app_args: &[&str]) -> Result<String, Error> {
    let mut xcrun_args: Vec<&str> = vec!["simctl", "launch", "-w", dev_id, app_id];
    xcrun_args.extend(app_args);
    debug!("Launching app via xcrun using args: {:?}", xcrun_args);
    let launch_output = Command::new(XCRUN).args(&xcrun_args).output()?;
    let launch_output = String::from_utf8_lossy(&launch_output.stdout);
    // Output from the launch command should be "Dinghy: $PID" which is after the 8th character.

    Ok(launch_output.split_at(8).1.to_string())
}

pub fn list_booted_simulators() -> Result<Vec<Device>, Error> {
    let simctl = Simctl::new();
    let devices = simctl.list().map_err(|err| anyhow!("{:?}", err))?;
    Ok(devices
        .devices()
        .into_iter()
        .available()
        .filter(|d| d.state == DeviceState::Booted)
        .cloned()
        .collect())
}
