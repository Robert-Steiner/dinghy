use std::path::{Path, PathBuf};

use anyhow::Error;
use cargo::{
    core::Workspace,
    ops::Packages,
    util::{important_paths::find_root_manifest_for_wd, interning::InternedString},
};
use log::debug;

pub fn profile(release: bool) -> InternedString {
    if release {
        InternedString::new("release")
    } else {
        InternedString::new("debug")
    }
}

pub fn find_root_manifest(path: &Path) -> Result<PathBuf, Error> {
    let manifest = find_root_manifest_for_wd(path)?;
    debug!("manifest path: {:?}", manifest);
    Ok(manifest)
}

pub fn packages_from_workspace(ws: &Workspace) -> Result<Packages, Error> {
    let spec = if ws.is_virtual() {
        debug!("workspace detected");
        Packages::from_flags(true, Vec::new(), Vec::new())?
    } else {
        Packages::Packages(Vec::new())
    };
    Ok(spec)
}
