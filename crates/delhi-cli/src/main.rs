//! The `delhi` command-line tool.
#![deny(missing_docs)]

mod cmd;
mod style;

fn usage() -> i32 {
    eprintln!(
        "delhi — an epistemic model checker

USAGE:
    delhi check <FILE>
    delhi state <FILE>              facts, and each agent's attitudes
    delhi show  <FILE>              the model itself, in the explicit form
    delhi eval  <FILE> -f <FORMULA>
    delhi ask   <FILE> [-d DEPTH] [-a ACTION]... -q <PATTERN>
    delhi step  <FILE> -a <ACTION>...
    delhi dot   <FILE>
    delhi repl  <FILE>
    delhi bench <FILE> [-n CYCLES] -a <ACTION>...

EXIT CODES:
    0  success, or the formula holds
    1  the file was rejected, or the formula does not hold
    2  usage error, or a malformed formula"
    );
    2
}

fn read(path: &str) -> Result<String, i32> {
    std::fs::read_to_string(path).map_err(|e| {
        eprintln!("{path}: {e}");
        2
    })
}

/// Entry point.
fn main() {
    // Once, before anything writes: colour only for a real terminal, and never when
    // the user has asked for none. Everything downstream reads a flag, so piping
    // `delhi dot … | dot -Tpng` stays byte-clean.
    if style::detect() {
        style::enable();
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("check") | Some("show") | Some("state") if args.len() == 2 => {
            match read(&args[1]) {
                Err(c) => c,
                Ok(src) => {
                    let mut out = String::new();
                    let c = match args[0].as_str() {
                        "check" => cmd::cmd_check(&src, &mut out),
                        "state" => cmd::cmd_state(&src, &mut out),
                        _ => cmd::cmd_show(&src, &mut out),
                    };
                    print!("{out}");
                    c
                }
            }
        }
        Some("eval") if args.len() == 4 && args[2] == "-f" => match read(&args[1]) {
            Err(c) => c,
            Ok(src) => {
                let mut out = String::new();
                let c = cmd::cmd_eval(&src, &args[3], &mut out);
                print!("{out}");
                c
            }
        },
        Some("dot") if args.len() == 2 => match read(&args[1]) {
            Err(c) => c,
            Ok(src) => {
                let mut out = String::new();
                let c = cmd::cmd_dot(&src, &mut out);
                print!("{out}");
                c
            }
        },
        Some("step") if args.len() >= 4 && args[2] == "-a" => match read(&args[1]) {
            Err(c) => c,
            Ok(src) => {
                let acts: Vec<String> = args[3..].to_vec();
                let mut out = String::new();
                let c = cmd::cmd_step(&src, &acts, &mut out);
                print!("{out}");
                c
            }
        },
        // `ask <FILE> [-d N] [-a ACTION]... -q <PATTERN>`. Flags are scanned rather than
        // positional, because `-a` takes a variable number of values and `-q` must be
        // able to follow them.
        Some("ask") if args.len() >= 4 => {
            let mut depth = 0usize;
            let mut pattern = String::new();
            let mut acts: Vec<String> = Vec::new();
            let mut i = 2;
            let mut bad = None;
            while i < args.len() {
                match args[i].as_str() {
                    "-d" if i + 1 < args.len() => match args[i + 1].parse() {
                        Ok(d) => {
                            depth = d;
                            i += 2;
                        }
                        Err(_) => {
                            bad = Some(format!("-d needs a number, got `{}`", args[i + 1]));
                            break;
                        }
                    },
                    "-q" if i + 1 < args.len() => {
                        pattern = args[i + 1].clone();
                        i += 2;
                    }
                    "-a" => {
                        i += 1;
                        while i < args.len() && !args[i].starts_with('-') {
                            acts.push(args[i].clone());
                            i += 1;
                        }
                    }
                    other => {
                        bad = Some(format!("unexpected argument `{other}`"));
                        break;
                    }
                }
            }
            match (bad, pattern.is_empty()) {
                (Some(msg), _) => {
                    eprintln!("{msg}");
                    2
                }
                (None, true) => usage(),
                (None, false) => match read(&args[1]) {
                    Err(c) => c,
                    Ok(src) => {
                        let mut out = String::new();
                        let c = cmd::cmd_ask(&src, &acts, &pattern, depth, &mut out);
                        print!("{out}");
                        c
                    }
                },
            }
        }
        Some("repl") if args.len() == 2 => match read(&args[1]) {
            Err(c) => c,
            Ok(src) => cmd::cmd_repl(&src),
        },
        // `bench <FILE> [-n N] -a <ACTION>...`. The `-n` is optional and, when given,
        // must precede `-a`, since everything after `-a` is an action name.
        Some("bench") if args.len() >= 4 => {
            let parsed = if args[2] == "-n" {
                match args[3].parse::<usize>() {
                    Ok(n) if n > 0 => Some((n, &args[4..])),
                    _ => None,
                }
            } else {
                Some((10, &args[2..]))
            };
            match parsed {
                None => {
                    eprintln!("-n needs a positive number, got `{}`", args[3]);
                    2
                }
                Some((_, rest)) if rest.first().map(String::as_str) != Some("-a") => usage(),
                Some((_, rest)) if rest.len() < 2 => usage(),
                Some((cycles, rest)) => match read(&args[1]) {
                    Err(c) => c,
                    Ok(src) => {
                        let acts: Vec<String> = rest[1..].to_vec();
                        let mut out = String::new();
                        let c = cmd::cmd_bench(&src, &acts, cycles, &mut out);
                        print!("{out}");
                        c
                    }
                },
            }
        }
        _ => usage(),
    };
    std::process::exit(code);
}
