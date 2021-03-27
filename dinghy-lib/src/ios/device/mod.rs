pub mod physical;
pub mod simulator;
mod utils;

use semver::Version;
// pub struct App {
//     name: String,
//     arch: ,
//     version: Version,
// }

// impl App {
//     pub fn new<N: Into<String>>(name: N, arch: physical::CpuArch, version: Version) -> Self {
//         Self {
//             name: name.into(),
//             arch,
//             version,
//         }
//     }

//     pub fn bundle_name(&self) -> String {
//         format!("{}.app", self.name)
//     }

//     pub fn target_triple(&self) -> String {
//         format!("{}-apple-ios", self.arch)
//     }
// }

pub struct AppBundle {
    executable: String,    // Dinghy
    app_bundle_id: String, // Dinghy
    bundle_id: String,     // robertt.debug.com.Dinghy.Dinghy
    version: Version,
}
