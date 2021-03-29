use dinghy_lib::ios::compiler::TestConfig;
use structopt::StructOpt;

#[derive(StructOpt)]
pub enum Options {
    Bench {},
    Test(Test),
}

#[derive(StructOpt)]
pub struct Test {
    /// Build artifacts in release mode, with optimizations
    #[structopt(long)]
    pub release: bool,

    /// Activate all available features
    #[structopt(long = "all-features")]
    pub all_features: bool,

    /// Do not activate the `default` feature
    #[structopt(long = "no-default-features")]
    pub no_default_features: bool,

    /// Space-separated list of features to activate
    #[structopt(value_delimiter = ",")]
    #[structopt(long, default_value = "")]
    pub features: Vec<String>,

    /// Build for the target triples
    #[structopt(long)]
    pub target: String,
}

impl From<Test> for TestConfig {
    fn from(opt: Test) -> Self {
        Self {
            release: opt.release,
            target: opt.target,
            all_features: opt.all_features,
            no_default_features: opt.no_default_features,
            features: opt.features,
        }
    }
}
