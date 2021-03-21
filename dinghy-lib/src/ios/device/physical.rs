use std::fmt::Formatter;
use std::{fmt, path::PathBuf};
use std::{fmt::Display, ptr};
use std::{mem, process};

use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::CFData;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation_sys::number::kCFBooleanTrue;

use crate::errors::*;
use crate::ios::mobiledevice_sys::*;
use crate::ios::xcode;
use crate::ios::IosPlatform;
use crate::project::Project;
use crate::Build;
use crate::BuildBundle;
use crate::Device;
use crate::DeviceCompatibility;
use crate::Runnable;

use super::utils::*;

#[derive(Clone, Debug)]
pub struct IosDevice {
    pub id: String,
    pub name: String,
    ptr: *const am_device,
    arch_cpu: &'static str,
    rustc_triple: String,
}

unsafe impl Send for IosDevice {}

impl IosDevice {
    pub fn new(ptr: *const am_device) -> Result<IosDevice> {
        let _session = ensure_session(ptr)?;
        let name = match device_read_value(ptr, "DeviceName")? {
            Some(Value::String(s)) => s,
            x => bail!("DeviceName should have been a string, was {:?}", x),
        };
        let cpu = match device_read_value(ptr, "CPUArchitecture")? {
            Some(Value::String(ref v)) if v == "arm64" || v == "arm64e" => "aarch64",
            _ => "armv7",
        };
        let id = if let Value::String(id) = rustify(unsafe { AMDeviceCopyDeviceIdentifier(ptr) })? {
            id
        } else {
            bail!("unexpected id format")
        };
        Ok(IosDevice {
            ptr,
            name,
            id,
            arch_cpu: cpu,
            rustc_triple: format!("{}-apple-ios", cpu),
        })
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

impl Device for IosDevice {
    fn clean_app(&self, _build_bundle: &BuildBundle) -> Result<()> {
        unimplemented!()
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
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
        run_ios_deploy(args, envs, &build_bundle.bundle_dir)?;
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
            run_ios_deploy(args, envs, &build_bundle.bundle_dir)?;
            build_bundles.push(build_bundle)
        }
        Ok(build_bundles)
    }
}

fn run_ios_deploy(args: &[&str], envs: &[&str], bundle_dir: &PathBuf) -> Result<()> {
    let exit = process::Command::new("ios-deploy")
        .args(&["--args", &args.join(" ")])
        .args(&["--envs", &envs.join(" ")])
        .args(&["--noninteractive", " --debug", "--bundle"])
        .arg(bundle_dir)
        .status()?;
    if exit.success() {
        Ok(())
    } else {
        bail!("failed with exit code: {}", exit)
    }
}

impl Display for IosDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Ok(fmt.write_str(
            format!(
                "IosDevice {{ \"id\": \"{}\", \"name\": {}, \"arch_cpu\": {} }}",
                self.id, self.name, self.arch_cpu
            )
            .as_str(),
        )?)
    }
}

impl DeviceCompatibility for IosDevice {
    fn is_compatible_with_ios_platform(&self, platform: &IosPlatform) -> bool {
        if platform.sim {
            false
        } else {
            platform.toolchain.rustc_triple == self.rustc_triple.as_str()
        }
    }
}

struct Session(*const am_device);

fn ensure_session(dev: *const am_device) -> Result<Session> {
    unsafe {
        mk_result(AMDeviceConnect(dev))?;
        if AMDeviceIsPaired(dev) == 0 {
            bail!("lost pairing")
        };
        mk_result(AMDeviceValidatePairing(dev))?;
        mk_result(AMDeviceStartSession(dev))?;
        Ok(Session(dev))
        // debug!("ensure session 4 ({:x})", rv);
        // if rv as u32 == 0xe800001d {
        // Ok(Session(::std::ptr::null()))
        // } else {
        // mk_result(rv)?;
        // Ok(Session(dev))
        // }
        //
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                if let Err(e) = mk_result(AMDeviceStopSession(self.0)) {
                    debug!("Error closing session {:?}", e);
                }
                if let Err(e) = mk_result(AMDeviceDisconnect(self.0)) {
                    error!("Error disconnecting {:?}", e);
                }
            }
        }
    }
}

fn mk_result(rv: i32) -> Result<()> {
    if rv as u32 == 0xe80000e2 {
        bail!("error: Device is locked. ({:x})", rv)
    } else if rv as u32 == 0xe80000be {
        bail!("error: 0xe80000be, kAMDMismatchedApplicationIdentifierEntitlementError: This application's application-identifier entitlement does not match that of the installed application. These values must match for an upgrade to be allowed. Help: check that the xcode project you created has \"Dinghy\" as Project Name, and make the prefix (Organisation identifier) something reasonably unique.")
    } else if rv as u32 == 0xe8000087 {
        bail!("error: 0xe8000087, Architecture mismatch")
    } else if rv as u32 == 0xe8008015 {
        bail!("error: 0xe8008015, A valid provisioning profile for this executable was not found.")
    } else if rv as u32 == 0xe8008016 {
        bail!("error: 0xe8008016, The executable was signed with invalid entitlements.")
    } else if rv as u32 == 0xe8008022 {
        bail!(
            "error: 0xe8000022, kAMDInvalidServiceError. (This one is relatively hard to diagnose. Try erasing the Dinghy app from the phone, rebooting the device, the computer, check for ios and xcode updates.)",
        )
    } else if rv as u32 == 0xe800007f {
        bail!("error: e800007f, The device OS version is too low.")
    } else if rv as u32 == 0xe8000007 {
        bail!("error: e8000007: Invalid argument.")
    } else if rv != 0 {
        bail!("error: {:x}", rv)
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug)]
enum Value {
    String(String),
    Data(Vec<u8>),
    I64(i64),
    Boolean(bool),
}

fn device_read_value(dev: *const am_device, key: &str) -> Result<Option<Value>> {
    unsafe {
        let key = CFString::new(key);
        let raw = AMDeviceCopyValue(dev, ptr::null(), key.as_concrete_TypeRef());
        if raw.is_null() {
            return Ok(None);
        }
        Ok(Some(rustify(raw)?))
    }
}

fn rustify(raw: CFTypeRef) -> Result<Value> {
    unsafe {
        let cftype: CFType = TCFType::wrap_under_get_rule(mem::transmute(raw));
        if cftype.type_of() == CFString::type_id() {
            let value: CFString =
                TCFType::wrap_under_get_rule(raw as *const core_foundation::string::__CFString);
            return Ok(Value::String(value.to_string()));
        }

        if cftype.type_of() == CFData::type_id() {
            let value: CFData =
                TCFType::wrap_under_get_rule(raw as *const core_foundation::data::__CFData);
            return Ok(Value::Data(value.bytes().to_vec()));
        }
        if cftype.type_of() == CFNumber::type_id() {
            let value: CFNumber =
                TCFType::wrap_under_get_rule(raw as *const core_foundation::number::__CFNumber);
            if let Some(i) = value.to_i64() {
                return Ok(Value::I64(i));
            }
        }
        if cftype.type_of() == CFBoolean::type_id() {
            return Ok(Value::Boolean(raw == kCFBooleanTrue as *const libc::c_void));
        }
        cftype.show();
        bail!("unknown value")
    }
}
