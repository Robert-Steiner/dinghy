use anyhow::Error;
use dinghy_lib::ios::compiler::compile_test;
use env_logger::{self, Env};
use log;
use structopt::StructOpt;

mod cli;

use cli::Options;

fn main() -> Result<(), Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();

    let opt = Options::from_args();

    match opt {
        Options::Bench {} => {}
        Options::Test(test_opt) => {
            compile_test(&test_opt.into())?;
            ()
        }
    };

    Ok(())
}
