use std::{fs, io::Write, process::Command};

use anyhow::{anyhow, Error, Result};
use log::{debug, trace};
use openssl::{nid::Nid, x509::X509};
use plist;
use serde::{Deserialize, Serialize};

use crate::{ios::tools::security, BuildBundle};

#[derive(Debug, Clone)]
pub struct SignatureSettings {
    pub identity: SigningIdentity,
    pub file: String,
    pub entitlements: String,
    pub name: String,
    pub profile: String,
}

#[derive(Debug, Clone)]
pub struct SigningIdentity {
    pub id: String,
    pub name: String,
    pub team: String,
}

#[derive(Deserialize, Debug)]
struct MobileProvision {
    #[serde(rename = "ProvisionedDevices")]
    provisioned_devices: Vec<String>,
    #[serde(rename = "TeamIdentifier")]
    team_identifier: Vec<String>,
    #[serde(rename = "Name")]
    name: String,
}

#[derive(Clone, Debug, Serialize)]

pub struct InfoPlist<'a> {
    #[serde(rename = "CFBundleExecutable")]
    cf_bundle_executable: &'a str,
    #[serde(rename = "CFBundleIdentifier")]
    cf_bundle_identifier: &'a str,
    #[serde(rename = "UIRequiredDeviceCapabilities")]
    ui_required_device_capabilities: Vec<&'a str>,
    #[serde(rename = "CFBundleVersion")]
    cf_bundle_version: &'a str,
    #[serde(rename = "CFBundleShortVersionString")]
    cf_bundle_short_version_string: &'a str,
}

pub fn create_plist_for_app(bundle: &BuildBundle, arch: &str, app_bundle_id: &str) -> Result<()> {
    let plist = fs::File::create(bundle.bundle_dir.join("Info.plist"))?;
    plist::to_writer_xml(
        plist,
        &InfoPlist {
            cf_bundle_executable: "Dinghy",
            cf_bundle_identifier: app_bundle_id,
            ui_required_device_capabilities: vec![arch],
            cf_bundle_version: "1",
            cf_bundle_short_version_string: "1.0",
        },
    )
    .map_err(|err| anyhow!(err))
}

pub fn sign_app(bundle: &BuildBundle, settings: &SignatureSettings) -> Result<()> {
    debug!(
        "Will sign {:?} with team: {} using key: {} and profile: {}",
        bundle.bundle_dir, settings.identity.team, settings.identity.name, settings.file
    );

    let entitlements = bundle.root_dir.join("entitlements.xcent");
    debug!("entitlements file: {}", entitlements.to_str().unwrap_or(""));
    let mut plist = fs::File::create(&entitlements)?;
    writeln!(plist, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(
        plist,
        r#"<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">"#
    )?;
    writeln!(plist, r#"<plist version="1.0"><dict>"#)?;
    writeln!(plist, "{}", settings.entitlements)?;
    writeln!(plist, r#"</dict></plist>"#)?;

    Command::new("codesign")
        .args(&["-s", &*settings.identity.name, "--entitlements"])
        .arg(entitlements)
        .arg(&bundle.bundle_dir)
        .status()?;
    Ok(())
}

pub fn look_for_signature_settings(device_id: &str) -> Result<Vec<SignatureSettings>> {
    let identity_regex = ::regex::Regex::new(r#"^ *[0-9]+\) ([A-Z0-9]{40}) "(.+)"$"#)?;

    let mut identities: Vec<SigningIdentity> = vec![];
    let find_identities = security::find_identities()?.stdout;
    for line in String::from_utf8(find_identities)?.split('\n') {
        if let Some(caps) = identity_regex.captures(&line) {
            let name: String = caps[2].into();
            if !name.starts_with("iPhone Developer: ") && !name.starts_with("Apple Development:") {
                continue;
            }

            let subject = get_subject(&name)?;

            identities.push(SigningIdentity {
                id: caps[1].into(),
                name: caps[2].into(),
                team: subject.to_string(),
            })
        }
    }

    debug!("signing identities: {:?}", identities);
    let mut settings = vec![];
    for file in fs::read_dir(
        dirs::home_dir()
            .expect("can't get HOME dir")
            .join("Library/MobileDevice/Provisioning Profiles"),
    )? {
        let file = file?;
        if file.path().starts_with(".")
            || file
                .path()
                .extension()
                .map(|ext| ext.to_string_lossy() != "mobileprovision")
                .unwrap_or(true)
        {
            trace!("skipping profile (?) {:?}", file.path());
            continue;
        }

        debug!("considering profile {:?}", file.path());
        let decoded = security::decode_cms(&file.path())?.stdout;
        let mobile_provision: MobileProvision = plist::from_bytes(&decoded)?;
        debug!("{:?}", mobile_provision);

        if !mobile_provision
            .provisioned_devices
            .contains(&device_id.to_string())
        {
            debug!("  no device match in profile");
            continue;
        }

        if !mobile_provision.name.ends_with("Dinghy") {
            debug!(
                "  app in profile does not match ({})",
                mobile_provision.name
            );
            continue;
        }

        // TODO: check date in future
        let team = mobile_provision
            .team_identifier
            .first()
            .ok_or_else(|| anyhow!("empty TeamIdentifier"))?;

        let identity = identities.iter().find(|i| &i.team == team);
        if identity.is_none() {
            debug!("no identity for team");
            continue;
        }
        let identity = identity.unwrap();
        let entitlements = String::from_utf8(decoded)?
            .split('\n')
            .skip_while(|line| !line.contains("<key>Entitlements</key>"))
            .skip(2)
            .take_while(|line| !line.contains("</dict>"))
            .collect::<Vec<&str>>()
            .join("\n");

        debug!("{}", entitlements);

        let file = file
            .path()
            .to_str()
            .ok_or_else(|| anyhow!("filename should be utf8"))?
            .to_string();

        settings.push(SignatureSettings {
            entitlements,
            file: file.clone(),
            name: mobile_provision.name,
            identity: identity.clone(),
            profile: file.clone(),
        });
    }
    Ok(settings)
}

fn get_subject(name: &str) -> Result<String, Error> {
    let cert = security::find_certificate(name)?.stdout;
    let x509 = X509::from_pem(&cert)?;
    let subject = x509
        .subject_name()
        .entries_by_nid(Nid::ORGANIZATIONALUNITNAME)
        .next()
        .unwrap()
        .data()
        .as_utf8()?;
    Ok(subject.to_string())
}
