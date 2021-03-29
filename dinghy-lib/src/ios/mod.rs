use std::sync::Arc;

use anyhow::Result;

use crate::{
    ios::tools::{ios_deploy, xcrun},
    Compiler,
    Device,
    Platform,
    PlatformManager,
};

pub use self::{
    device::{physical::Physical, simulator::Simulator},
    platform::IosPlatform,
};

mod command_ext;
mod device;

pub mod compiler;
mod platform;
mod tools;
mod xcode;

pub struct IosManager {
    compiler: Arc<Compiler>,
}

impl IosManager {
    pub fn new(compiler: Arc<Compiler>) -> Result<Option<IosManager>> {
        Ok(Some(IosManager { compiler }))
    }
}

impl PlatformManager for IosManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        let mut devices = Vec::new();

        if let Some(device) = ios_deploy::list_device()? {
            devices.push(Box::new(Physical::new(device)) as Box<dyn Device>)
        }

        let simulators = xcrun::list_booted_simulators()?
            .into_iter()
            .map(|sim| Box::new(Simulator::new(sim)) as Box<dyn Device>);

        devices.extend(simulators);

        Ok(devices)
    }

    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>> {
        ["aarch64", "x86_64"]
            .iter()
            .map(|arch| {
                let id = format!("auto-ios-{}", arch);
                let rustc_triple = format!("{}-apple-ios", arch);
                IosPlatform::new(
                    id,
                    &rustc_triple,
                    &self.compiler,
                    crate::config::PlatformConfiguration::default(),
                )
                .map(|pf| pf as Box<dyn Platform>)
            })
            .collect()
    }
}
