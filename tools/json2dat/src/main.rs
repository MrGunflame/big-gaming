mod convert;
mod types;

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
}

fn main() {
    let args = Args::parse();

    if let Err(err) = convert_file(&args.input, &args.output) {
        eprintln!("conversion error: {}", err);
        std::process::exit(1);
    }
}

fn convert_file(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open(input)?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    drop(file);

    let data = serde_json::from_slice(&mut buf)?;
    let buf = convert::encode(data);

    let mut file = File::create(output)?;
    file.write_all(&buf)?;

    Ok(())
}
