use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Synchronize the lyrics")]
pub struct Args {
    /// For safety reasons, this programs list actions by default, this flag makes the program take those actions.
    #[clap(long, short = 'x')]
    pub execute: bool,

    /// Source directory of the subtitles.
    pub source: PathBuf,

    /// Container of the target directories of the subtitles.
    pub target: PathBuf,
}
