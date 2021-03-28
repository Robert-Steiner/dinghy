use std::sync::Arc;

use anyhow::Result;
use log::debug;

use crate::{
    ios::tools::{ios_deploy, xcrun},
    Compiler,
    Device,
    Platform,
    PlatformManager,
};

use self::compiler::compile_test;
pub use self::{
    device::{physical::Physical, simulator::Simulator},
    platform::IosPlatform,
};

mod command_ext;
mod device;

mod compiler;
mod platform;
mod tools;
mod xcode;

use compiler::Test;

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
        let test = Test {
            release: true,
            targets: vec![String::from("aarch64-apple-ios")],
            all_features: false,
            no_default_features: false,
            features: vec![],
        };
        compile_test(test);

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
