use std::process;

use crate::device::make_remote_app_with_name;
use crate::errors::*;
use crate::ios::xcode;
use crate::project::Project;
use crate::Build;
use crate::BuildBundle;
use crate::Runnable;

pub fn make_ios_app(
    project: &Project,
    build: &Build,
    runnable: &Runnable,
    app_id: &str,
) -> Result<BuildBundle> {
    use crate::project;
    let build_bundle = make_remote_app_with_name(project, build, runnable, Some("Dinghy.app"))?;
    project::rec_copy(&runnable.exe, build_bundle.bundle_dir.join("Dinghy"), false)?;
    let magic = process::Command::new("file")
        .arg(
            runnable
                .exe
                .to_str()
                .ok_or_else(|| anyhow!("path conversion to string: {:?}", runnable.exe))?,
        )
        .output()?;
    let magic = String::from_utf8(magic.stdout)?;
    let target = magic
        .split(" ")
        .last()
        .ok_or_else(|| anyhow!("empty magic"))?;
    xcode::add_plist_to_app(&build_bundle, target, app_id)?;
    Ok(build_bundle)
}
