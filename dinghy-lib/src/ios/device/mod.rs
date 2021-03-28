pub mod physical;
pub mod simulator;
mod utils;

use semver::Version;

pub struct AppBundle {
    executable: String,    // Dinghy
    app_bundle_id: String, // Dinghy
    bundle_id: String,     // robertt.debug.com.Dinghy.Dinghy
    version: Version,
}
