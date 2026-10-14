use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use yohash_core::algorithm::{algorithm_by_name, DEFAULT_ALGORITHM};
use yohash_core::engine::HashEngine;
use yohash_core::format::OutputFormat;

#[derive(Parser)]
struct Args {
    #[arg(required = true)]
    files: Vec<PathBuf>,
    #[arg(short = 'a', long, default_value = DEFAULT_ALGORITHM)]
    alg: String,
    #[arg(short = 'f', long, default_value = "upper", value_parser = parse_format)]
    format: OutputFormat,
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    OutputFormat::parse(value).ok_or_else(|| {
        format!("invalid format '{value}'; expected upper, lowercase, or base64")
    })
}

fn main() -> ExitCode {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            let _ = error.print();
            return ExitCode::from(1);
        }
    };
    let Some(algorithm) = algorithm_by_name(&args.alg) else {
        eprintln!("yohash: unknown algorithm: {}", args.alg);
        return ExitCode::from(1);
    };

    let engine = HashEngine::new();
    if args.files.len() == 1 {
        match engine.hash_file(&args.files[0], algorithm, args.format) {
            Ok(hash) => {
                println!("{hash}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("yohash: {error}");
                ExitCode::from(1)
            }
        }
    } else {
        let mut failed = false;
        for (path, result) in engine.hash_files(&args.files, algorithm, args.format) {
            match result {
                Ok(hash) => println!("{}: {hash}", path.display()),
                Err(error) => {
                    eprintln!("yohash: {error}");
                    failed = true;
                }
            }
        }
        if failed {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        }
    }
}
