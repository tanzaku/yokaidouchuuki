use once_cell::sync::Lazy;
use structopt::StructOpt;

use crate::domain::to_charcode_indices;

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(long)]
    pub prefix: Option<String>,

    #[structopt(long)]
    pub suffix: Option<String>,

    #[structopt(long)]
    pub verbose: bool,

    #[structopt(long)]
    pub ignore_cache: bool,
    // #[structopt(long)]
    // pub contains: Option<String>,
}

pub struct OptInternal {
    pub prefix: Option<Vec<usize>>,

    pub suffix: Option<Vec<usize>>,

    pub verbose: bool,

    pub ignore_cache: bool,
}

pub static OPT: Lazy<OptInternal> = Lazy::new(|| {
    let opt = Opt::from_args();
    OptInternal {
        prefix: opt.prefix.as_ref().map(|s| to_charcode_indices(s)),
        suffix: opt.suffix.as_ref().map(|s| to_charcode_indices(s)),
        verbose: opt.verbose,
        ignore_cache: opt.ignore_cache,
    }
});
