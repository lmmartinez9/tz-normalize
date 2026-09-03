mod normalize;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::process;

fn main() {
    let mut to_utc = false;
    let mut output_path: Option<String> = None;
    let mut paths: Vec<String> = Vec::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--to-utc" {
            to_utc = true;
        } else if arg == "--output" {
            match args.next() {
                Some(path) => output_path = Some(path),
                None => {
                    eprintln!("--output requires a file path");
                    process::exit(1);
                }
            }
        } else if let Some(path) = arg.strip_prefix("--output=") {
            output_path = Some(path.to_string());
        } else {
            paths.push(arg);
        }
    }

    let mut out: Box<dyn Write> = match &output_path {
        Some(path) => match File::create(path) {
            Ok(file) => Box::new(file),
            Err(e) => {
                eprintln!("{}: {}", path, e);
                process::exit(1);
            }
        },
        None => Box::new(io::stdout()),
    };
    let mut error_count = 0usize;

    if paths.is_empty() {
        let stdin = io::stdin();
        process_reader(stdin.lock(), "stdin", to_utc, &mut out, &mut error_count);
    } else {
        for path in &paths {
            if path == "-" {
                let stdin = io::stdin();
                process_reader(stdin.lock(), "stdin", to_utc, &mut out, &mut error_count);
                continue;
            }

            match File::open(path) {
                Ok(file) => process_reader(
                    BufReader::new(file),
                    path,
                    to_utc,
                    &mut out,
                    &mut error_count,
                ),
                Err(e) => {
                    eprintln!("{}: {}", path, e);
                    error_count += 1;
                }
            }
        }
    }

    if error_count > 0 {
        process::exit(1);
    }
}

fn process_reader<R: BufRead, W: Write>(
    reader: R,
    source: &str,
    to_utc: bool,
    out: &mut W,
    error_count: &mut usize,
) {
    for (i, line) in reader.lines().enumerate() {
        let line_num = i + 1;
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("{}:{}: {}", source, line_num, e);
                *error_count += 1;
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let result = if to_utc {
            normalize::normalize_line_to_utc(&line)
        } else {
            normalize::normalize_line(&line)
        };

        match result {
            Ok(normalized) => {
                let _ = writeln!(out, "{}", normalized);
            }
            Err(e) => {
                eprintln!("{}:{}: {}", source, line_num, e);
                *error_count += 1;
            }
        }
    }
}
