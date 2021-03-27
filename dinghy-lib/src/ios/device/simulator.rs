use std::{
    fmt,
    fmt::{Display, Formatter},
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process::Command,
    u32,
};

use anyhow::{anyhow, bail, Error, Result};
use log::debug;
use simctl::{get_app_container::Container, Device as SimDevice};
use tempdir::TempDir;
use tinytemplate::TinyTemplate;

use crate::{
    ios::{
        tools::{lldb, xcrun},
        IosPlatform,
    },
    project::Project,
    Build,
    BuildBundle,
    Device,
    DeviceCompatibility,
    Runnable,
};

use super::utils::*;

#[derive(Clone, Debug)]
pub struct Simulator {
    device: SimDevice,
}

impl Simulator {
    pub fn new(device: SimDevice) -> Self {
        Self { device }
    }

    fn install_app(
        &self,
        project: &Project,
        build: &Build,
        runnable: &Runnable,
    ) -> Result<BuildBundle> {
        let build_bundle = Simulator::make_app(project, build, runnable)?;

        self.device.uninstall("Dinghy").map_err(|_| anyhow!(""))?;
        let path = build_bundle.bundle_dir.clone();
        self.device
            .install(path.as_path())
            .map_err(|_| anyhow!(""))?;

        Ok(build_bundle)
    }

    fn make_app(project: &Project, build: &Build, runnable: &Runnable) -> Result<BuildBundle> {
        make_ios_app(project, build, runnable, "Dinghy")
    }
}

impl Device for Simulator {
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
        let install_path = self
            .device
            .get_app_container("Dinghy", &Container::App)
            .map_err(|_| anyhow!(""))?;

        launch_lldb_simulator(
            &self,
            install_path.to_str().ok_or(anyhow!(""))?,
            args,
            envs,
            true,
        )?;
        Ok(build_bundle)
    }

    fn id(&self) -> &str {
        &self.device.udid
    }

    fn name(&self) -> &str {
        &self.device.name
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

impl Display for Simulator {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_fmt(format_args!(
            "Simulator {{ \"id\": \"{}\", \"name\": {} }}",
            self.id(),
            self.name(),
        ))
    }
}

impl DeviceCompatibility for Simulator {
    fn is_compatible_with_ios_platform(&self, platform: &IosPlatform) -> bool {
        platform.sim && platform.toolchain.rustc_triple == "x86_64-apple-ios"
    }
}

fn launch_app(dev: &Simulator, app_args: &[&str], _envs: &[&str]) -> Result<()> {
    let dinghy_pid = xcrun::launch_app(dev.id(), "Dinghy", app_args)?;
    debug!("dinghy_pid {:?}", dinghy_pid);
    let (lldb_path, guard) = create_lldb_script(&dinghy_pid)?;
    let output = lldb::run_source(&lldb_path)?;
    guard.close()?;
    extract_lldb_exit_status(&output.stdout).map(|_| ())
}

fn launch_lldb_simulator(
    dev: &Simulator,
    installed: &str,
    args: &[&str],
    envs: &[&str],
    debugger: bool,
) -> Result<()> {
    let dir = TempDir::new("mobiledevice-rs-lldb")?;
    let tmppath = dir.path();
    let lldb_script_filename = tmppath.join("lldb-script");
    {
        let python_lldb_support = tmppath.join("helpers.py");
        let helper_py = include_str!("../helpers.py");
        let helper_py = helper_py.replace("ENV_VAR_PLACEHOLDER", &envs.join("\", \""));
        fs::File::create(&python_lldb_support)?.write_fmt(format_args!("{}", &helper_py))?;
        let mut script = File::create(&lldb_script_filename)?;

        let mut tt = TinyTemplate::new();
        tt.add_template("lldb_script", TEMPLATE)?;
        let context = Context {
            installed: installed.to_string(),
            python_lldb_support: python_lldb_support.to_string_lossy().to_string(),
            debugger,
            id: dev.id().to_string(),
            args: args.join(" "),
        };
        let rendered = tt.render("lldb_script", &context)?;
        script.write_all(rendered.as_bytes())?;
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

fn create_lldb_script(app_pid: &str) -> Result<(PathBuf, TempDir), Error> {
    // Attaching to the processes needs to be done in a script, not a
    // commandline parameter or lldb will say "no simulators found".
    let temp_dir = TempDir::new("mobiledevice-rs-lldb")?;
    let path = temp_dir.path().join("lldb-script");

    let mut file = File::create(&path)?;
    file.write_fmt(format_args!(
        include_str!("../templates/lldb.tmpl"),
        app_pid = app_pid,
    ))?;

    debug!("lldb-script path: {:?}", path);
    Ok((path, temp_dir))
}

fn extract_lldb_exit_status(stdout: &Vec<u8>) -> Result<u32, Error> {
    let output = String::from_utf8_lossy(stdout).to_string();

    debug!("LLDB output:\n{}", output);
    /*
    The stdout from lldb is something like:

    (lldb) attach 34163
    Process 34163 stopped
    * thread #1, stop reason = signal SIGSTOP
        frame #0: 0x00000001019cd000 dyld`_dyld_start
    dyld`_dyld_start:
    ->  0x1019cd000 <+0>: popq   %rdi
        0x1019cd001 <+1>: pushq  x0
        0x1019cd003 <+3>: movq   %rsp, %rbp
        0x1019cd006 <+6>: andq   $-0x10, %rsp
    Target 0: (Dinghy) stopped.

    Executable module set to .....
    Architecture set to: x86_64h-apple-ios-.
    (lldb) continue
    Process 34163 resuming
    Process 34163 exited with status = 101 (0x00000065)

    (lldb) quit

    We need the "exit with status" line which is the 3rd from the last
    */
    let exit_status_line: Option<&str> = output.lines().rev().skip(3).next();
    let tokens: Vec<&str> = if let Some(exit_status_line) = exit_status_line {
        exit_status_line.split_whitespace().rev().collect()
    } else {
        bail!(
            "Failed to get the exit status line from lldb: {:?}",
            exit_status_line
        );
    };

    if let Some(exit_status) = tokens.get(1) {
        exit_status.parse::<u32>().map_err(|err| anyhow!("{}", err))
    } else {
        bail!(
            "Failed to parse lldb exit line for an exit status. {:?}",
            tokens
        )
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
target create {installed}
script pass
command script import "{python_lldb_support}"
command script add -s synchronous -f helpers.start start
command script add -f helpers.connect_command connect
connect connect://{id}
"#;

// if !debugger {
//     writeln!(script, "start {}", args.join(" "))?;
//     writeln!(script, "quit")?;
// }
