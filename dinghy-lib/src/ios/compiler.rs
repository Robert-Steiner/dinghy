use std::ffi::OsString;

use anyhow::Error;
use cargo::{
    core::{
        compiler::{BuildConfig, CompileMode},
        Workspace,
    },
    ops::{compile, CompileFilter, CompileOptions, FilterRule, LibRule},
    util::{important_paths::find_root_manifest_for_wd, interning::InternedString},
    Config,
};
use log::debug;
use structopt::StructOpt;

#[derive(StructOpt)]
enum Invocation {
    Bench {},
    Build {
        #[structopt(long)]
        release: bool,
    },
    Test(Test),
}

#[derive(StructOpt)]
pub struct Test {
    /// Build artifacts in release mode, with optimizations
    #[structopt(long)]
    pub release: bool,

    /// Build for the target triples
    #[structopt(long, value_name = "TARGET1,TARGET2")]
    #[structopt(value_delimiter = ",")]
    #[structopt(default_value = "aarch64-apple-ios,x86_64-apple-ios")]
    pub targets: Vec<String>,

    /// Activate all available features
    #[structopt(long = "all-features")]
    pub all_features: bool,

    /// Do not activate the `default` feature
    #[structopt(long = "no-default-features")]
    pub no_default_features: bool,

    /// Space-separated list of features to activate
    #[structopt(long)]
    #[structopt(value_delimiter = ",")]
    pub features: Vec<String>,
}

fn compiler_options(invocation: Invocation) {
    match invocation {
        Invocation::Bench {} => {}
        Invocation::Build { release } => {}

        Invocation::Test(t) => {}
    }
}

pub fn compile_test(requested: Test) -> Result<(), Error> {
    let config = Config::default()?;
    let mut build_conf = BuildConfig::new(&config, None, &requested.targets, CompileMode::Test)?;
    build_conf.requested_profile = profile(requested.release);

    let manifest = find_root_manifest_for_wd(config.cwd())?;
    debug!("Manifest path: {:?}", manifest);

    let ws = Workspace::new(&manifest, &config)?;

    let mut compile_opts = CompileOptions::new(&config, CompileMode::Test)?;
    compile_opts.features = requested.features;
    compile_opts.all_features = requested.all_features;
    compile_opts.no_default_features = requested.no_default_features;
    compile_opts.build_config = build_conf;

    compile_opts.filter = CompileFilter::new(
        LibRule::Default,   // compile the library, so the unit tests can be run filtered
        FilterRule::All, // compile the binaries, so the unit tests in binaries can be run filtered
        FilterRule::All, // compile the tests, so the integration tests can be run filtered
        FilterRule::none(), // specify --examples to unit test binaries filtered
        FilterRule::none(), // specify --benches to unit test benchmarks filtered
    );

    let res = compile(&ws, &compile_opts)?;
    debug!("{:?}", res.tests.first().unwrap().path);

    Ok(())
}

fn profile(release: bool) -> InternedString {
    if release {
        InternedString::new("release")
    } else {
        InternedString::new("debug")
    }
}
