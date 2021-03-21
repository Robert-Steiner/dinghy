use std::sync;

use anyhow::Result;

use crate::{Compiler, Configuration, Device, Platform, PlatformConfiguration, PlatformManager};

pub use self::{device::HostDevice, platform::HostPlatform};

mod device;
mod platform;

pub struct HostManager {
    compiler: sync::Arc<Compiler>,
    host_conf: PlatformConfiguration,
}

impl HostManager {
    pub fn probe(compiler: sync::Arc<Compiler>, conf: &Configuration) -> Option<HostManager> {
        let host_conf = conf
            .platforms
            .get("host")
            .map(|it| (*it).clone())
            .unwrap_or_else(PlatformConfiguration::empty);
        HostManager {
            compiler,
            host_conf,
        }
        .into()
    }

    fn platform(&self) -> Result<HostPlatform> {
        platform::HostPlatform::new(sync::Arc::clone(&self.compiler), self.host_conf.clone())
    }
}

impl PlatformManager for HostManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        Ok(vec![Box::new(HostDevice::new(
            self.platform()?,
            &self.compiler,
        ))])
    }

    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>> {
        Ok(vec![Box::new(self.platform()?)])
    }
}
