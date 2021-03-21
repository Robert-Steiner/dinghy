use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::path::Path;
use std::process;

use anyhow::{anyhow, bail, Result};
use log::debug;
use tinytemplate::TinyTemplate;

use crate::ios::IosPlatform;
use crate::project::Project;
use crate::Build;
use crate::BuildBundle;
use crate::Device;
use crate::DeviceCompatibility;
use crate::Runnable;

use super::utils::*;

#[derive(Clone, Debug)]
pub struct IosSimDevice {
    pub id: String,
    pub name: String,
    pub os: String,
}

impl IosSimDevice {
    fn install_app(
        &self,
        project: &Project,
        build: &Build,
        runnable: &Runnable,
    ) -> Result<BuildBundle> {
        let build_bundle = IosSimDevice::make_app(project, build, runnable)?;
        let _ = process::Command::new("xcrun")
            .args(&["simctl", "uninstall", &self.id, "Dinghy"])
            .status()?;
        let stat = process::Command::new("xcrun")
            .args(&[
                "simctl",
                "install",
                &self.id,
                build_bundle
                    .bundle_dir
                    .to_str()
                    .ok_or_else(|| anyhow!("conversion to string"))?,
            ])
            .status()?;
        if stat.success() {
            Ok(build_bundle)
        } else {
            bail!(
                "Failed to install {} for {}",
                runnable.exe.display(),
                self.id
            )
        }
    }

    fn make_app(project: &Project, build: &Build, runnable: &Runnable) -> Result<BuildBundle> {
        make_ios_app(project, build, runnable, "Dinghy")
    }
}

impl Device for IosSimDevice {
    fn clean_app(&self, _build_bundle: &BuildBundle) -> Result<()> {
        unimplemented!()
    }

    fn debug_app(
        &self,
        project: &Project,
        build: &Build,
        args: &[&str],
        envs: &[&str],
    ) -> Result<BuildBundle> {
        let runnable = build
            .runnables
            .get(0)
            .ok_or_else(|| anyhow!("No executable compiled"))?;
        let build_bundle = self.install_app(project, build, runnable)?;
        let install_path = String::from_utf8(
            process::Command::new("xcrun")
                .args(&["simctl", "get_app_container", &self.id, "Dinghy"])
                .output()?
                .stdout,
        )?;
        launch_lldb_simulator(&self, &install_path, args, envs, true)?;
        Ok(build_bundle)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn run_app(
        &self,
        project: &Project,
        build: &Build,
        args: &[&str],
        envs: &[&str],
    ) -> Result<Vec<BuildBundle>> {
        let mut build_bundles = Vec::with_capacity(build.runnables.len());
        for runnable in build.runnables.iter() {
            let build_bundle = self.install_app(project, build, runnable)?;
            launch_app(&self, args, envs)?;
            build_bundles.push(build_bundle);
        }
        Ok(build_bundles)
    }
}

impl Display for IosSimDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_fmt(format_args!(
            "IosSimDevice {{ \"id\": \"{}\", \"name\": {}, \"os\": {} }}",
            self.id, self.name, self.os
        ))
    }
}

impl DeviceCompatibility for IosSimDevice {
    fn is_compatible_with_ios_platform(&self, platform: &IosPlatform) -> bool {
        platform.sim && platform.toolchain.rustc_triple == "x86_64-apple-ios"
    }
}

fn launch_app(dev: &IosSimDevice, app_args: &[&str], _envs: &[&str]) -> Result<()> {
    use std::io::Write;
    let dir = ::tempdir::TempDir::new("mobiledevice-rs-lldb")?;
    let tmppath = dir.path();
    let mut install_path = String::from_utf8(
        process::Command::new("xcrun")
            .args(&["simctl", "get_app_container", &dev.id, "Dinghy"])
            .output()?
            .stdout,
    )?;
    install_path.pop();
    let stdout = Path::new(&install_path)
        .join("stdout")
        .to_string_lossy()
        .into_owned();
    let stdout_param = &format!("--stdout={}", stdout);
    let mut xcrun_args: Vec<&str> = vec!["simctl", "launch", "-w", stdout_param, &dev.id, "Dinghy"];
    xcrun_args.extend(app_args);
    debug!("Launching app via xcrun using args: {:?}", xcrun_args);
    let launch_output = process::Command::new("xcrun").args(&xcrun_args).output()?;
    let launch_output = String::from_utf8_lossy(&launch_output.stdout);

    // Output from the launch command should be "Dinghy: $PID" which is after the 8th character.
    let dinghy_pid = launch_output.split_at(8).1;

    // Attaching to the processes needs to be done in a script, not a commandline parameter or
    // lldb will say "no simulators found".
    let lldb_script_filename = tmppath.join("lldb-script");
    let mut script = fs::File::create(&lldb_script_filename)?;
    writeln!(script, "attach {}", dinghy_pid)?;
    writeln!(script, "continue")?;
    writeln!(script, "quit")?;
    let output = process::Command::new("lldb")
        .arg("")
        .arg("-s")
        .arg(lldb_script_filename)
        .output()?;
    let test_contents = std::fs::read_to_string(stdout)?;
    println!("{}", test_contents);

    let output: String = String::from_utf8_lossy(&output.stdout).to_string();
    debug!("LLDB OUTPUT: {}", output);
    // The stdout from lldb is something like:
    //
    // (lldb) attach 34163
    // Process 34163 stopped
    // * thread #1, stop reason = signal SIGSTOP
    //     frame #0: 0x00000001019cd000 dyld`_dyld_start
    // dyld`_dyld_start:
    // ->  0x1019cd000 <+0>: popq   %rdi
    //     0x1019cd001 <+1>: pushq  $0x0
    //     0x1019cd003 <+3>: movq   %rsp, %rbp
    //     0x1019cd006 <+6>: andq   $-0x10, %rsp
    // Target 0: (Dinghy) stopped.
    //
    // Executable module set to .....
    // Architecture set to: x86_64h-apple-ios-.
    // (lldb) continue
    // Process 34163 resuming
    // Process 34163 exited with status = 101 (0x00000065)
    //
    // (lldb) quit
    //
    // We need the "exit with status" line which is the 3rd from the last
    let lines: Vec<&str> = output.lines().rev().collect();
    let exit_status_line = lines.get(2);
    if let Some(exit_status_line) = exit_status_line {
        let words: Vec<&str> = exit_status_line.split_whitespace().rev().collect();
        if let Some(exit_status) = words.get(1) {
            let exit_status = exit_status.parse::<u32>()?;
            if exit_status == 0 {
                Ok(())
            } else {
                panic!("Non-zero exit code from lldb: {}", exit_status);
            }
        } else {
            panic!(
                "Failed to parse lldb exit line for an exit status. {:?}",
                words
            );
        }
    } else {
        panic!("Failed to get the exit status line from lldb: {:?}", lines);
    }
}

fn launch_lldb_simulator(
    dev: &IosSimDevice,
    installed: &str,
    args: &[&str],
    envs: &[&str],
    debugger: bool,
) -> Result<()> {
    use std::io::Write;
    use std::process::Command;
    let dir = ::tempdir::TempDir::new("mobiledevice-rs-lldb")?;
    let tmppath = dir.path();
    let lldb_script_filename = tmppath.join("lldb-script");
    {
        let python_lldb_support = tmppath.join("helpers.py");
        let helper_py = include_str!("../helpers.py");
        let helper_py = helper_py.replace("ENV_VAR_PLACEHOLDER", &envs.join("\", \""));
        fs::File::create(&python_lldb_support)?.write_fmt(format_args!("{}", &helper_py))?;
        let mut script = fs::File::create(&lldb_script_filename)?;

        let mut tt = TinyTemplate::new();
        tt.add_template("lldb_script", TEMPLATE)?;
        let context = Context {
            installed: installed.to_string(),
            python_lldb_support: python_lldb_support.to_string_lossy().to_string(),
            debugger,
            id: dev.id.clone(),
            args: args.join(" "),
        };
        let rendered = tt.render("lldb_script", &context)?;
        script.write_all(rendered.as_bytes())?;

        // std::thread::sleep(std::time::Duration::from_secs(1223423));

        // writeln!(script, "platform select ios-simulator")?;
        // writeln!(script, "target create {}", installed)?;
        // writeln!(script, "script pass")?;
        // writeln!(script, "command script import {:?}", python_lldb_support)?;
        // writeln!(
        //     script,
        //     "command script add -s synchronous -f helpers.start start"
        // )?;
        // writeln!(
        //     script,
        //     "command script add -f helpers.connect_command connect"
        // )?;
        // writeln!(script, "connect connect://{}", dev.id)?;
        // if !debugger {
        //     writeln!(script, "start {}", args.join(" "))?;
        //     writeln!(script, "quit")?;
        // }
    }

    let stat = Command::new("xcrun")
        .arg("lldb")
        .arg("-Q")
        .arg("-s")
        .arg(lldb_script_filename)
        .status()?;
    if stat.success() {
        Ok(())
    } else {
        bail!("LLDB returned error code {:?}", stat.code())
    }
}

use serde::Serialize;

#[derive(Serialize)]
struct Context {
    installed: String,
    python_lldb_support: String,
    debugger: bool,
    id: String,
    args: String,
}

static TEMPLATE: &'static str = r#"
platform select ios-simulator
target create  {installed}
script pass
command script import {python_lldb_support}
command script add -s synchronous -f helpers.start start
command script add -f helpers.connect_command connect
connect connect://{id}
{{ if debugger }}
start {args}
quit
{{ endif }}
"#;
