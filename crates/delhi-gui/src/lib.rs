//! A local web UI for exploring delhi files: editor, state view, graph, console.
//!
//! Binds to loopback only. It is a single-user debugging tool, not a service — there is
//! no authentication and none is wanted, so it must not be exposed beyond the machine
//! running it.
//!
//! The entry point is [`serve`], reached from the CLI as `delhi gui`.
#![deny(missing_docs)]

mod api;
mod builtin;

use std::path::{Path, PathBuf};

pub use builtin::BUILTIN;

/// The page, embedded so the binary is self-contained.
const PAGE: &str = include_str!("ui.html");

/// Prefix marking a bundled example in the file list.
///
/// Bundled files are read-only and live in the binary; everything without this prefix is
/// a real file in the served directory. Keeping the two namespaces distinct is what lets
/// a user open `examples/coin_lie.delhi`, edit it, and save it beside their own work
/// without the question of which one they just overwrote.
const BUILTIN_PREFIX: &str = "examples/";

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

/// Whether `name` is a plain `.delhi` filename.
///
/// Load-bearing, because names arrive from the query string: without it,
/// `?name=../../../etc/passwd` would read — or, for save, *write* — whatever the process
/// can reach. The served directory is now the user's own, which changes what is at stake
/// but not the rule: a request may name a file *in* that directory, never a path out of
/// it.
fn is_plain_delhi(name: &str) -> bool {
    !name.is_empty()
        && name.ends_with(".delhi")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && name.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// Every file the UI offers: the served directory's own `.delhi` files first, then the
/// bundled examples under [`BUILTIN_PREFIX`].
///
/// The user's files come first because they are what the user came for; the bundles are
/// a reference shelf underneath.
fn file_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| is_plain_delhi(n))
        .collect();
    names.sort();
    names.extend(BUILTIN.iter().map(|(n, _)| format!("{BUILTIN_PREFIX}{n}")));
    names
}

/// Reads one file: a bundled example, or a `.delhi` file directly in the served
/// directory. Anything else is refused.
fn read_source(root: &Path, path: &str) -> Option<String> {
    if let Some(name) = path.strip_prefix(BUILTIN_PREFIX) {
        return BUILTIN.iter().find(|(n, _)| *n == name).map(|(_, src)| (*src).to_string());
    }
    is_plain_delhi(path).then(|| std::fs::read_to_string(root.join(path)).ok())?
}

/// Writes `src` to `<root>/<name>`.
///
/// A bundled example cannot be written to: it lives in the binary, and a save that
/// appeared to succeed and then vanished on restart would be worse than a refusal. Save
/// it under its own name instead and it lands in the served directory like any other.
fn save_source(root: &Path, name: &str, src: &str) -> Result<String, String> {
    if name.starts_with(BUILTIN_PREFIX) {
        return Err(format!("`{name}` is a bundled example — save it under a new name"));
    }
    if !is_plain_delhi(name) {
        return Err(format!("`{name}` is not a plain .delhi filename"));
    }
    std::fs::write(root.join(name), src).map_err(|e| format!("{name}: {e}"))?;
    Ok(name.to_string())
}

/// A path as a person would write it.
///
/// Windows `canonicalize` returns an extended-length path — `\\?\C:\...` — which is
/// correct and unreadable. The prefix is only meaningful to the API that produced it, and
/// this string is shown to a user who wants to confirm which directory they are editing.
fn display_path(p: &Path) -> String {
    let s = p.display().to_string();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

/// Serves the UI on `127.0.0.1:port`, offering the `.delhi` files in `root`.
///
/// Blocks until the process is interrupted. `root` is created if it does not exist, so
/// `delhi gui notes/` works the first time as well as the second.
pub fn serve(port: u16, root: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(root)?;
    // Canonicalised for the banner only: the user should be able to read back exactly
    // which directory they are about to edit, not a relative path they have to resolve.
    let root: PathBuf = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let addr = format!("127.0.0.1:{port}");
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| std::io::Error::other(format!("could not bind {addr}: {e}")))?;

    println!("delhi ui   http://{addr}");
    println!("serving    {}", display_path(&root));
    println!("           {} bundled example(s); ctrl-c to stop", BUILTIN.len());

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
            "/api/root" => ("text/plain; charset=utf-8", display_path(&root)),
            "/api/files" => (
                "application/json",
                serde_json::to_string(&file_names(&root)).expect("serialises"),
            ),
            "/api/file" => (
                "text/plain; charset=utf-8",
                read_source(&root, &get("name")).unwrap_or_default(),
            ),
            "/api/save" => {
                let reply = match save_source(&root, &get("name"), &body) {
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch directory under the target dir, unique per test.
    fn tmp(tag: &str) -> PathBuf {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/t").join(tag);
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("scratch dir");
        p
    }

    #[test]
    fn repeated_keys_keep_their_order_so_a_trace_survives() {
        // The action trace is order-sensitive: announce-then-peek is a different state
        // from peek-then-announce. A map-based parse would lose that.
        let pairs = query_pairs("a=tell()&a=look()&f=B%5Ba%5D+h");
        let trace: Vec<&str> =
            pairs.iter().filter(|(k, _)| k == "a").map(|(_, v)| v.as_str()).collect();
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
    fn the_served_directory_is_listed_alongside_the_bundled_examples() {
        let root = tmp("listing");
        std::fs::write(root.join("mine.delhi"), "x").unwrap();
        std::fs::write(root.join("notes.txt"), "x").unwrap();
        let names = file_names(&root);
        assert_eq!(names[0], "mine.delhi", "the user's own files come first: {names:?}");
        assert!(!names.iter().any(|n| n.ends_with(".txt")));
        assert!(names.contains(&"examples/coin_lie.delhi".to_string()), "{names:?}");
        assert_eq!(names.len(), 1 + BUILTIN.len());
    }

    #[test]
    fn a_bundled_example_reads_from_the_binary_and_not_from_disk() {
        // The point of embedding: this works with `root` pointing at an empty directory,
        // which is what a fresh download of the binary actually has.
        let root = tmp("bundled");
        let src = read_source(&root, "examples/coin_lie.delhi").expect("bundled");
        assert!(src.contains("announce_not_heads"));
        assert!(read_source(&root, "examples/nope.delhi").is_none());
    }

    #[test]
    fn a_file_loads_and_a_traversal_does_not() {
        let root = tmp("guards");
        std::fs::write(root.join("ok.delhi"), "hello").unwrap();
        assert_eq!(read_source(&root, "ok.delhi").as_deref(), Some("hello"));

        // Names arrive from the query string, so every guard here is load-bearing.
        for bad in [
            "../Cargo.toml",
            "../../Cargo.toml",
            "/etc/passwd",
            "sub/ok.delhi",
            "ok.delhi/../../Cargo.toml",
            "Cargo.toml",
            "examples/../../Cargo.toml",
        ] {
            assert!(read_source(&root, bad).is_none(), "`{bad}` must be refused");
        }
    }

    #[test]
    fn saving_writes_into_the_served_directory_and_nowhere_else() {
        let root = tmp("save");
        assert_eq!(save_source(&root, "new.delhi", "body").unwrap(), "new.delhi");
        assert_eq!(std::fs::read_to_string(root.join("new.delhi")).unwrap(), "body");

        // Save writes to disk, so the same guard matters more here than for reading.
        // These are rejected before any filesystem call.
        for bad in ["../escape.delhi", "sub/dir.delhi", "..delhi", "notes.txt", ""] {
            assert!(save_source(&root, bad, "x").is_err(), "`{bad}` must be refused");
        }
        // A bundled example is in the binary; a write that seemed to land and then
        // reverted on restart would be worse than saying no.
        assert!(save_source(&root, "examples/coin_lie.delhi", "x").is_err());
        assert!(!root.join("..").join("escape.delhi").exists());
    }

    #[test]
    fn every_example_in_the_repository_is_bundled() {
        // Guards the one way this list rots: an example added to `examples/` that nobody
        // remembers to embed is missing from every downloaded binary, and the omission is
        // invisible from inside the repository, where the directory is right there.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let mut on_disk: Vec<String> = std::fs::read_dir(dir)
            .expect("examples/")
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".delhi"))
            .collect();
        on_disk.sort();
        let mut bundled: Vec<String> = BUILTIN.iter().map(|(n, _)| n.to_string()).collect();
        bundled.sort();
        assert_eq!(bundled, on_disk, "BUILTIN in builtin.rs is out of step with examples/");
    }
}
