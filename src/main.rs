mod normalize;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut error_count = 0usize;

    if args.is_empty() {
        let stdin = io::stdin();
        process_reader(stdin.lock(), "stdin", &mut out, &mut error_count);
    } else {
        for path in &args {
            if path == "-" {
                let stdin = io::stdin();
                process_reader(stdin.lock(), "stdin", &mut out, &mut error_count);
                continue;
            }

            match File::open(path) {
                Ok(file) => process_reader(BufReader::new(file), path, &mut out, &mut error_count),
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

        match normalize::normalize_line(&line) {
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
