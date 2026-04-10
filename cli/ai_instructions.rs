use clap::Parser;
use pipe_trait::Pipe;
use std::{
    fmt,
    fs::{read_to_string, File},
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const SHARED: &str = include_str!("../template/ai-instructions/shared.md");
const CLAUDE: &str = include_str!("../template/ai-instructions/claude.md");
const COPILOT: &str = include_str!("../template/ai-instructions/copilot.md");
const AGENTS: &str = include_str!("../template/ai-instructions/agents.md");

#[derive(Clone, Copy)]
struct Fragments(&'static [&'static str]);

impl fmt::Display for Fragments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Fragments(fragments) = self;
        for fragment in *fragments {
            f.write_str(fragment)?;
        }
        Ok(())
    }
}

impl Fragments {
    fn matches(&self, actual: &str) -> bool {
        let Fragments(fragments) = self;
        let mut remaining = actual;
        for fragment in *fragments {
            match remaining.strip_prefix(fragment) {
                Some(rest) => remaining = rest,
                None => return false,
            }
        }
        remaining.is_empty()
    }
}

const FILES: &[(&str, Fragments)] = &[
    ("CLAUDE.md", Fragments(&[SHARED, CLAUDE])),
    (
        ".github/copilot-instructions.md",
        Fragments(&[SHARED, COPILOT]),
    ),
    ("AGENTS.md", Fragments(&[SHARED, AGENTS])),
];

enum RuntimeError {
    WriteFile {
        path: &'static str,
        error: io::Error,
    },
    ReadFile {
        path: &'static str,
        error: io::Error,
    },
    Outdated,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::WriteFile { path, error } => {
                write!(f, "Failed to write {path}: {error}")
            }
            RuntimeError::ReadFile { path, error } => {
                write!(f, "Failed to read {path}: {error}")
            }
            RuntimeError::Outdated => write!(f, "Some AI instruction files were outdated."),
        }
    }
}

impl RuntimeError {
    fn hint(&self, args: &Args) -> Option<impl fmt::Display> {
        match self {
            RuntimeError::ReadFile { .. } | RuntimeError::WriteFile { .. } => None,
            RuntimeError::Outdated => Some(format!(
                "Run `cargo run --features ai-instructions --bin lyrics-ai-instructions -- --generate {}` to update.",
                args.repository.display(),
            )),
        }
    }
}

/// Check or generate AI instruction files from templates.
#[derive(Debug, Parser)]
struct Args {
    /// Generate the AI instruction files instead of checking them.
    #[clap(long)]
    generate: bool,

    /// Path to the top-level directory of the repository.
    repository: PathBuf,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let result = match args.generate {
        true => write_files(&args.repository),
        false => check_files(&args.repository),
    };
    if let Err(error) = result {
        eprintln!("error: {error}");
        if let Some(hint) = error.hint(&args) {
            eprintln!("hint: {hint}");
        }
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn write_files(repository: &Path) -> Result<(), RuntimeError> {
    for (path, fragments) in FILES {
        let mut output = repository
            .join(path)
            .pipe(File::create)
            .map_err(|error| RuntimeError::WriteFile { path, error })?;
        write!(output, "{fragments}").map_err(|error| RuntimeError::WriteFile { path, error })?;
        eprintln!("info: Generated file {path}");
    }
    Ok(())
}

fn check_files(repository: &Path) -> Result<(), RuntimeError> {
    let mut result: Result<(), RuntimeError> = Ok(());
    for &(path, fragments) in FILES {
        let actual = repository
            .join(path)
            .pipe(read_to_string)
            .map_err(|error| RuntimeError::ReadFile { path, error })?;
        if !fragments.matches(&actual) {
            eprintln!("error: File {path} is out-of-date");
            result = Err(RuntimeError::Outdated);
        }
    }
    result
}
