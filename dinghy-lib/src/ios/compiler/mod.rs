mod test;
pub(crate) mod util;

use std::path::PathBuf;

use cargo::core::Package;
pub use test::{compile_test, TestConfig};

pub struct BuildUnit {
    pub executable_path: PathBuf,
    pub package: Package,
    pub target: String,
}

pub enum Target {
    Anarch64,
    X86_64,
}

impl Target {
    pub fn as_str(&self) -> &str {
        match self {
            Target::Anarch64 => "aarch64",
            Target::X86_64 => "x86_64",
        }
    }
}
