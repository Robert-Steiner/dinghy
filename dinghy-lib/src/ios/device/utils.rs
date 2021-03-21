use std::process;

use anyhow::{anyhow, Result};

use crate::{
    device::make_remote_app_with_name,
    ios::xcode,
    project,
    project::Project,
    Build,
    BuildBundle,
    Runnable,
};

pub fn make_ios_app(
    project: &Project,
    build: &Build,
    runnable: &Runnable,
    app_id: &str,
) -> Result<BuildBundle> {
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
        .split(' ')
        .last()
        .ok_or_else(|| anyhow!("empty magic"))?
        .trim();
    xcode::add_plist_to_app(&build_bundle, target, app_id)?;
    Ok(build_bundle)
}
