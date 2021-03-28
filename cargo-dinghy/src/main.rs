use std::{env, env::current_dir, sync::Arc};

use anyhow::{anyhow, bail, Result};
use clap::ArgMatches;
use dinghy_lib::{
    compiler::Compiler,
    config::dinghy_config,
    project::Project,
    utils::arg_as_string_vec,
    Build,
    Device,
    Dinghy,
    Platform,
};
use itertools::Itertools;
use log::{debug, error, info};

use crate::cli::CargoDinghyCli;

mod cli;

fn main() {
    let filtered_args = env::args()
        .enumerate()
        .filter(|&(ix, ref s)| !(ix == 1 && s == "dinghy"))
        .map(|(_, s)| s);
    let matches = CargoDinghyCli::parse(filtered_args);

    if env::var("RUST_LOG").is_err() {
        let dinghy_verbosity =
            match matches.occurrences_of("VERBOSE") - matches.occurrences_of("QUIET") {
                0 => "info",
                1 => "debug",
                _ => "trace",
            };
        env::set_var(
            "RUST_LOG",
            format!(
                "cargo_dinghy={},dinghy={}",
                dinghy_verbosity, dinghy_verbosity
            ),
        );
    };
    env_logger::init();

    if let Err(e) = run_command(&matches) {
        error!("{:?}", e);
        // positively ugly.
        if e.to_string().contains("are filtered out on platform") {
            std::process::exit(3)
        } else {
            std::process::exit(1)
        }
    }
}

fn run_command(args: &ArgMatches) -> Result<()> {
    let conf = Arc::new(dinghy_config(current_dir().unwrap())?);

    let compiler = Arc::new(Compiler::from_args(args.subcommand().1.unwrap_or(args))?);
    let dinghy = Dinghy::probe(&conf, &compiler)?;
    let project = Project::new(&conf);

    let (platform, device) = select_platform_and_device_from_cli(&args, &dinghy)?;
    info!(
        "Targeting platform '{}' and device '{}'",
        platform.id(),
        device.as_ref().map(|it| it.id()).unwrap_or("<none>")
    );

    match args.subcommand() {
        ("bench", Some(sub_args)) => prepare_and_run(device, project, platform, args, sub_args),
        ("build", Some(sub_args)) => build(&platform, &project, args, sub_args).and(Ok(())),
        ("clean", Some(_)) => compiler.clean(&**platform),
        ("test", Some(sub_args)) => prepare_and_run(device, project, platform, args, sub_args),
        (sub, _) => bail!("Unknown dinghy command '{}'", sub),
    }
}

fn build(
    platform: &Arc<Box<dyn Platform>>,
    project: &Project,
    args: &ArgMatches,
    sub_args: &ArgMatches,
) -> Result<Build> {
    let build_args = CargoDinghyCli::build_args_from(args);
    let build = platform.build(&project, &build_args)?;

    if sub_args.is_present("STRIP") {
        platform.strip(&build)?;
    }
    Ok(build)
}

fn prepare_and_run(
    device: Option<Arc<Box<dyn Device>>>,
    project: Project,
    platform: Arc<Box<dyn Platform>>,
    args: &ArgMatches,
    sub_args: &ArgMatches,
) -> Result<()> {
    use cargo_metadata::{CargoOpt, MetadataCommand};

    let _metadata = MetadataCommand::new()
        .manifest_path("./Cargo.toml")
        .features(CargoOpt::AllFeatures)
        .exec()
        .unwrap();

    println!("{:?}", _metadata);

    debug!("Build for {}", platform);
    let build = build(&platform.clone(), &project, args, sub_args)?;

    if sub_args.is_present("NO_RUN") {
        return Ok(());
    }

    debug!("Run on {:?}", device);
    let device = device.ok_or_else(|| anyhow!("No device found"))?;
    let args = arg_as_string_vec(sub_args, "ARGS");
    let envs = arg_as_string_vec(sub_args, "ENVS");

    let args = args.iter().map(|s| &s[..]).collect::<Vec<_>>();
    let envs = envs.iter().map(|s| &s[..]).collect::<Vec<_>>();
    let build_bundles = if sub_args.is_present("DEBUGGER") {
        debug!("Debug app");
        vec![device.debug_app(&project, &build, &*args, &*envs)?]
    } else {
        debug!("Run app");
        device.run_app(&project, &build, &*args, &*envs)?
    };

    if sub_args.is_present("CLEANUP") {
        for build_bundle in build_bundles {
            device.clean_app(&build_bundle)?;
        }
    }
    Ok(())
}

fn select_platform_and_device_from_cli(
    matches: &ArgMatches,
    dinghy: &Dinghy,
) -> Result<(Arc<Box<dyn Platform>>, Option<Arc<Box<dyn Device>>>)> {
    if let Some(platform_name) = matches.value_of("PLATFORM") {
        let platform = dinghy
            .platform_by_name(platform_name)
            .ok_or_else(|| anyhow!("No '{}' platform found", platform_name))?;

        let device = dinghy
            .devices()
            .into_iter()
            .filter(|device| {
                matches
                    .value_of("DEVICE")
                    .map(|filter| {
                        format!("{}", device)
                            .to_lowercase()
                            .contains(&filter.to_lowercase())
                    })
                    .unwrap_or(true)
            })
            .filter(|it| platform.is_compatible_with(&**it.as_ref()))
            .next();

        Ok((platform, device))
    } else if let Some(device_filter) = matches.value_of("DEVICE") {
        let is_banned_auto_platform_id = |id: &str| -> bool {
            id.contains("auto-android")
                && (id.contains("min") || id.contains("latest") || id.contains("api"))
        };
        let devices = dinghy
            .devices()
            .into_iter()
            .filter(move |it| {
                format!("{:?}", it)
                    .to_lowercase()
                    .contains(&device_filter.to_lowercase())
            })
            .collect_vec();
        if devices.len() == 0 {
            bail!("No devices found for name hint `{}'", device_filter)
        }
        devices
            .into_iter()
            .filter_map(|d| {
                let pf = dinghy
                    .platforms()
                    .iter()
                    .filter(|pf| !is_banned_auto_platform_id(&pf.id()))
                    .find(|pf| pf.is_compatible_with(&**d))
                    .cloned();
                debug!(
                    "Looking for platform for {}: found {:?}",
                    d.id(),
                    pf.as_ref().map(|p| p.id())
                );
                pf.map(|it| (it, Some(d)))
            })
            .next()
            .ok_or_else(|| {
                anyhow!(
                    "No device and platform combination found for device hint `{}'",
                    device_filter
                )
            })
    } else {
        Ok((dinghy.host_platform(), Some(dinghy.host_device())))
    }
}
