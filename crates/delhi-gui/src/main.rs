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

/// The examples directory, resolved relative to the repository root.
fn examples_dir() -> std::path::PathBuf {
    // `CARGO_MANIFEST_DIR` is `crates/delhi-gui`, so the repo root is two levels up.
    // Resolving at runtime rather than embedding lets edits to the files show up
    // without a rebuild.
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// Names of the `.delhi` files shipped in `examples/`, sorted.
fn example_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(examples_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.ends_with(".delhi"))
        .collect();
    names.sort();
    names
}

/// Reads one example, refusing any name that is not a plain `.delhi` filename.
///
/// The guard matters because the name arrives from the query string: without it,
/// `?name=../../../etc/passwd` would read whatever the process can.
fn example_source(name: &str) -> Option<String> {
    let plain = !name.is_empty()
        && name.ends_with(".delhi")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..");
    if !plain {
        return None;
    }
    std::fs::read_to_string(examples_dir().join(name)).ok()
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
            "/api/examples" => (
                "application/json",
                serde_json::to_string(&example_names()).expect("serialises"),
            ),
            "/api/example" => match example_source(&get("name")) {
                Some(src) => ("text/plain; charset=utf-8", src),
                None => ("text/plain; charset=utf-8", String::new()),
            },
            "/api/state" => ("application/json", api::state(&body, &trace)),
            "/api/eval" => ("application/json", api::eval(&body, &trace, &get("f"))),
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
    fn example_names_are_the_shipped_files() {
        let names = example_names();
        assert!(names.contains(&"coin_lie.delhi".to_string()), "got {names:?}");
        assert!(names.iter().all(|n| n.ends_with(".delhi")));
    }

    #[test]
    fn an_example_loads_and_a_traversal_does_not() {
        assert!(example_source("coin_lie.delhi").expect("loads").contains("announce_not_heads"));
        // The name comes from the query string, so the guard is load-bearing.
        assert!(example_source("../Cargo.toml").is_none());
        assert!(example_source("../../Cargo.toml").is_none());
        assert!(example_source("/etc/passwd").is_none());
        assert!(example_source("coin_lie.delhi/../../../Cargo.toml").is_none());
        assert!(example_source("Cargo.toml").is_none(), "only .delhi files");
    }
}
