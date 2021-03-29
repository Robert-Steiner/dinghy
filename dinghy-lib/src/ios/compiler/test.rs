use anyhow::Error;
use cargo::{
    core::{
        compiler::{BuildConfig, CompileMode, UnitOutput},
        Workspace,
    },
    ops::{compile, CompileFilter, CompileOptions, FilterRule, LibRule},
    Config,
};

use super::{
    util::{find_root_manifest, packages_from_workspace, profile},
    BuildUnit,
};

pub struct TestConfig {
    /// Build artifacts in release mode, with optimizations
    pub release: bool,

    /// Build for the target triples
    pub target: String,

    /// Activate all available features
    pub all_features: bool,

    /// Do not activate the `default` feature
    pub no_default_features: bool,

    /// Space-separated list of features to activate
    pub features: Vec<String>,
}

pub fn compile_test(requested: &TestConfig) -> Result<Vec<BuildUnit>, Error> {
    let config = Config::default()?;
    let manifest_root = find_root_manifest(config.cwd())?;
    let ws = Workspace::new(&manifest_root, &config)?;

    let mut build_conf = BuildConfig::new(
        &config,
        None,
        &[requested.target.clone()],
        CompileMode::Test,
    )?;
    build_conf.requested_profile = profile(requested.release);

    let mut compile_opts = CompileOptions::new(&config, CompileMode::Test)?;
    compile_opts.build_config = build_conf;
    compile_opts.spec = packages_from_workspace(&ws)?;
    compile_opts.features = requested.features.clone();
    compile_opts.all_features = requested.all_features;
    compile_opts.no_default_features = requested.no_default_features;

    compile_opts.filter = CompileFilter::new(
        LibRule::Default,   // compile the library, so the unit tests can be run filtered
        FilterRule::All, // compile the binaries, so the unit tests in binaries can be run filtered
        FilterRule::All, // compile the tests, so the integration tests can be run filtered
        FilterRule::none(), // specify --examples to unit test binaries filtered
        FilterRule::none(), // specify --benches to unit test benchmarks filtered
    );

    let compilation = compile(&ws, &compile_opts)?;

    let build_units: Vec<BuildUnit> = compilation
        .tests
        .into_iter()
        .map(|UnitOutput { unit, path, .. }| BuildUnit {
            executable_path: path,
            package: unit.pkg.clone(),
            target: requested.target.clone(),
        })
        .collect();

    Ok(build_units)
}
