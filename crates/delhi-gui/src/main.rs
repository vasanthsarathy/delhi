//! A local web UI for exploring delhi files: editor, state view, graph, console.
//!
//! Binds to loopback only. It is a single-user debugging tool, not a service — there is
//! no authentication and none is wanted, so it must not be exposed beyond the machine
//! running it.
#![deny(missing_docs)]

mod api;

/// The page, embedded so the binary is self-contained.
const PAGE: &str = include_str!("ui.html");

/// Splits a query string into `(key, value)` pairs, percent-decoded.
///
/// Repeated keys are kept in order, which is how the action trace arrives: `a=x&a=y`
/// means "apply x then y". A map would lose both the repetition and the order.
fn query_pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|s| !s.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(k), percent_decode(v))
        })
        .collect()
}

/// Decodes `%XX` escapes and `+` as space.
///
/// A malformed escape is passed through literally rather than dropped, so a stray `%`
/// in a formula shows up in the error message instead of silently changing the query.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => {
                let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    None => {
                        out.push(b[i]);
                        i += 1;
                    }
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The two directories files may be read from. Only the second is written to.
///
/// `examples/` is curated and version-controlled; `scratch/` is gitignored and is where
/// anything authored in the browser lands, so the UI cannot overwrite a shipped example.
const DIRS: [&str; 2] = ["examples", "scratch"];

/// Resolves one of [`DIRS`] relative to the repository root.
///
/// `CARGO_MANIFEST_DIR` is `crates/delhi-gui`, so the root is two levels up. Resolved at
/// runtime rather than embedded, so edits to a file show up without a rebuild.
fn dir_path(dir: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(dir)
}

/// Whether `name` is a plain `.delhi` filename.
///
/// Load-bearing, because names arrive from the query string: without it,
/// `?name=../../../etc/passwd` would read — or, for save, *write* — whatever the process
/// can reach.
fn is_plain_delhi(name: &str) -> bool {
    !name.is_empty()
        && name.ends_with(".delhi")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && name.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// Every `.delhi` file in both directories, as `dir/name`, sorted within each directory.
fn file_names() -> Vec<String> {
    let mut out = Vec::new();
    for dir in DIRS {
        let mut names: Vec<String> = std::fs::read_dir(dir_path(dir))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| is_plain_delhi(n))
            .collect();
        names.sort();
        out.extend(names.into_iter().map(|n| format!("{dir}/{n}")));
    }
    out
}

/// Splits a `dir/name` path, accepting only a known directory and a plain filename.
fn split_path(path: &str) -> Option<(&'static str, &str)> {
    let (dir, name) = path.split_once('/')?;
    let dir = DIRS.iter().find(|d| **d == dir)?;
    is_plain_delhi(name).then_some((*dir, name))
}

/// Reads one file, refusing anything outside the two known directories.
fn read_source(path: &str) -> Option<String> {
    let (dir, name) = split_path(path)?;
    std::fs::read_to_string(dir_path(dir).join(name)).ok()
}

/// Writes `src` to `scratch/<name>`, creating the directory if needed.
///
/// Only `scratch/` is writable. Accepting a directory from the request would let the UI
/// overwrite a curated example, and there is no undo here beyond git.
fn save_source(name: &str, src: &str) -> Result<String, String> {
    if !is_plain_delhi(name) {
        return Err(format!("`{name}` is not a plain .delhi filename"));
    }
    let dir = dir_path("scratch");
    std::fs::create_dir_all(&dir).map_err(|e| format!("scratch/: {e}"))?;
    std::fs::write(dir.join(name), src).map_err(|e| format!("scratch/{name}: {e}"))?;
    Ok(format!("scratch/{name}"))
}

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let addr = format!("127.0.0.1:{port}");
    let server = match tiny_http::Server::http(&addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("could not bind {addr}: {e}");
            std::process::exit(2);
        }
    };
    println!("delhi ui  ->  http://{addr}");
    println!("(loopback only; ctrl-c to stop)");

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
        let pairs = query_pairs(query);
        let trace: Vec<String> =
            pairs.iter().filter(|(k, _)| k == "a").map(|(_, v)| v.clone()).collect();
        let get = |key: &str| {
            pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()).unwrap_or_default()
        };

        let mut body = String::new();
        let _ = request.as_reader().read_to_string(&mut body);

        let (mime, payload) = match path {
            "/" => ("text/html; charset=utf-8", PAGE.to_string()),
            "/api/files" => (
                "application/json",
                serde_json::to_string(&file_names()).expect("serialises"),
            ),
            "/api/file" => match read_source(&get("name")) {
                Some(src) => ("text/plain; charset=utf-8", src),
                None => ("text/plain; charset=utf-8", String::new()),
            },
            "/api/save" => {
                let reply = match save_source(&get("name"), &body) {
                    Ok(path) => serde_json::json!({ "ok": true, "path": path }),
                    Err(e) => serde_json::json!({ "ok": false, "error": e }),
                };
                ("application/json", reply.to_string())
            }
            "/api/state" => ("application/json", api::state(&body, &trace)),
            "/api/eval" => ("application/json", api::eval(&body, &trace, &get("f"))),
            "/api/ask" => {
                let depth = get("d").parse().unwrap_or(0);
                ("application/json", api::ask(&body, &trace, &get("f"), depth))
            }
            _ => ("text/plain; charset=utf-8", "not found".to_string()),
        };

        let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], mime.as_bytes())
            .expect("valid header");
        let response = tiny_http::Response::from_string(payload).with_header(header);
        let _ = request.respond(response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_keys_keep_their_order_so_a_trace_survives() {
        // The action trace is order-sensitive: announce-then-peek is a different state
        // from peek-then-announce. A map-based parse would lose that.
        let pairs = query_pairs("a=tell()&a=look()&f=B%5Ba%5D+h");
        let trace: Vec<&str> = pairs.iter().filter(|(k, _)| k == "a").map(|(_, v)| v.as_str()).collect();
        assert_eq!(trace, vec!["tell()", "look()"]);
        assert_eq!(pairs.iter().find(|(k, _)| k == "f").unwrap().1, "B[a] h");
    }

    #[test]
    fn percent_decoding_handles_the_characters_formulas_actually_contain() {
        assert_eq!(percent_decode("B%5Balice%5D%20%21h"), "B[alice] !h");
        assert_eq!(percent_decode("move%28c%2Cr1%2Cr2%29"), "move(c,r1,r2)");
        // A trailing or malformed escape is passed through rather than swallowed, so a
        // typo surfaces in the error instead of silently altering the query.
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn file_names_are_qualified_by_directory() {
        let names = file_names();
        assert!(names.contains(&"examples/coin_lie.delhi".to_string()), "got {names:?}");
        assert!(names.iter().all(|n| n.ends_with(".delhi")));
        assert!(names.iter().all(|n| n.starts_with("examples/") || n.starts_with("scratch/")));
    }

    #[test]
    fn a_file_loads_and_a_traversal_does_not() {
        assert!(read_source("examples/coin_lie.delhi")
            .expect("loads")
            .contains("announce_not_heads"));
        // Names arrive from the query string, so every guard here is load-bearing.
        assert!(read_source("examples/../Cargo.toml").is_none());
        assert!(read_source("../Cargo.toml").is_none());
        assert!(read_source("/etc/passwd").is_none());
        assert!(read_source("examples/coin_lie.delhi/../../Cargo.toml").is_none());
        assert!(read_source("examples/Cargo.toml").is_none(), "only .delhi files");
        assert!(read_source("coin_lie.delhi").is_none(), "a directory is required");
        assert!(read_source("src/lib.delhi").is_none(), "and it must be a known one");
    }

    #[test]
    fn only_plain_names_may_be_saved() {
        // Save writes to disk, so the same guard matters more here than for reading.
        // These are rejected before any filesystem call, which is why the test can
        // assert on them without risking a stray file.
        for bad in ["../escape.delhi", "sub/dir.delhi", "..delhi", "notes.txt", ""] {
            assert!(save_source(bad, "x").is_err(), "`{bad}` must be refused");
        }
        assert!(is_plain_delhi("my_domain.delhi"));
        assert!(is_plain_delhi("v2-test.delhi"));
        assert!(!is_plain_delhi("space name.delhi"), "no spaces");
    }
}
