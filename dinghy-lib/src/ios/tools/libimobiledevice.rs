use std::{
    fmt::{self, Display, Formatter},
    process::Command,
};

use anyhow::Error;

use crate::ios::command_ext::OutputExt;

const I_DEVICE_INFO: &'static str = "ideviceinfo";

pub fn device_cpu_arch() -> Result<CpuArch, Error> {
    let output = Command::new(I_DEVICE_INFO)
        .args(&["-k", "CPUArchitecture"])
        .output()?
        .expect_success()?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string()
        .into())
}

pub fn device_id() -> Result<String, Error> {
    let output = Command::new(I_DEVICE_INFO)
        .args(&["-k", "UniqueDeviceID"])
        .output()?
        .expect_success()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn device_name() -> Result<String, Error> {
    let output = Command::new(I_DEVICE_INFO)
        .args(&["-k", "DeviceName"])
        .output()?
        .expect_success()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[derive(Debug, Clone)]
pub enum CpuArch {
    Aarch64,
    Unsupported(String),
}

impl CpuArch {
    pub fn to_string(&self) -> String {
        match self {
            CpuArch::Aarch64 => String::from("arm64"),
            CpuArch::Unsupported(inner) => inner.clone(),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            CpuArch::Aarch64 => "aarch64",
            CpuArch::Unsupported(inner) => inner,
        }
    }
}

impl From<String> for CpuArch {
    fn from(string: String) -> Self {
        match string.as_str() {
            "arm64" => Self::Aarch64,
            _ => Self::Unsupported(string),
        }
    }
}

impl Display for CpuArch {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_str(self.as_str())
    }
}
