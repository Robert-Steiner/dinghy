use std::{
    fmt,
    fmt::{Display, Formatter},
};

use anyhow::{anyhow, Result};

use crate::{
    ios::{
        tools::{ios_deploy, libimobiledevice, libimobiledevice::CpuArch},
        xcode,
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
pub struct Physical {
    id: String,
    name: String,
    cpu_arch: CpuArch,
}

impl Physical {
    pub fn new(id: String) -> Result<Physical> {
        let cpu_arch = libimobiledevice::device_cpu_arch()?;
        let name = libimobiledevice::device_name()?;

        Ok(Physical { name, id, cpu_arch })
    }

    fn make_app(
        &self,
        project: &Project,
        build: &Build,
        runnable: &Runnable,
    ) -> Result<BuildBundle> {
        let signing = xcode::look_for_signature_settings(&self.id)?
            .pop()
            .ok_or_else(|| anyhow!("no signing identity found"))?;
        let app_id = signing
            .name
            .split(' ')
            .last()
            .ok_or_else(|| anyhow!("no app id ?"))?;

        let build_bundle = make_ios_app(project, build, runnable, &app_id)?;

        xcode::sign_app(&build_bundle, &signing)?;
        Ok(build_bundle)
    }
}

impl Device for Physical {
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
        let build_bundle = self.make_app(project, build, runnable)?;
        ios_deploy::launch_app(args, envs, &build_bundle.bundle_dir)?;
        Ok(build_bundle)
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
            let build_bundle = self.make_app(project, build, runnable)?;
            ios_deploy::launch_app(args, envs, &build_bundle.bundle_dir)?;
            build_bundles.push(build_bundle)
        }
        Ok(build_bundles)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Display for Physical {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_fmt(format_args!(
            "Physical {{ \"id\": \"{}\", \"name\": {}, \"arch\": {} }}",
            self.id, self.name, self.cpu_arch
        ))
    }
}

impl DeviceCompatibility for Physical {
    fn is_compatible_with_ios_platform(&self, platform: &IosPlatform) -> bool {
        if platform.sim {
            false
        } else {
            platform.toolchain.rustc_triple == format!("{}-apple-ios", self.cpu_arch)
        }
    }
}
