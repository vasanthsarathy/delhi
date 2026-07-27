//! Reproduces the §6.2 measurement from `research/bisimulation/`. If these numbers
//! move, either the model generator or a bisimulation notion changed.

use delhi_mb::{blocks_dynamic, blocks_full, Model};

fn all_relations(n: usize) -> Vec<Vec<(usize, usize)>> {
    // (class, level) per world; every locally-well-preordered relation arises this way.
    let total = n.pow(n as u32);
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in 0..total {
        for l in 0..total {
            let mut spec = Vec::with_capacity(n);
            let (mut x, mut y) = (c, l);
            for _ in 0..n {
                spec.push((x % n, y % n));
                x /= n;
                y /= n;
            }
            let mut edges: Vec<(usize, usize)> = Vec::new();
            for u in 0..n {
                for v in 0..n {
                    if spec[u].0 == spec[v].0 && spec[u].1 <= spec[v].1 {
                        edges.push((u, v));
                    }
                }
            }
            if seen.insert(edges.clone()) {
                out.push(edges);
            }
        }
    }
    out
}

fn colourings(n: usize) -> Vec<Vec<usize>> {
    let total = n.pow(n as u32);
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in 0..total {
        let mut v = Vec::with_capacity(n);
        let mut x = c;
        for _ in 0..n {
            v.push(x % n);
            x /= n;
        }
        let mut map = std::collections::HashMap::new();
        let canon: Vec<usize> = v
            .iter()
            .map(|&k| {
                let next = map.len();
                *map.entry(k).or_insert(next)
            })
            .collect();
        if seen.insert(canon.clone()) {
            out.push(canon);
        }
    }
    out
}

#[test]
fn incompleteness_rate_matches_the_research_measurement() {
    let n = 3;
    let rels = all_relations(n);
    let cols = colourings(n);
    let mut models = 0u32;
    let mut incomplete = 0u32;
    let mut unsound = 0u32;

    for edges in &rels {
        for colour in &cols {
            let mut m = Model::new(n, 1, n);
            for (w, &c) in colour.iter().enumerate() {
                m.val[w].set(c);
            }
            for &(u, v) in edges {
                m.relate(0, u, v);
            }
            assert_eq!(m.validate(), Ok(()));
            models += 1;
            let br = blocks_dynamic(&m);
            let bd = blocks_full(&m);
            let mut gap = false;
            for u in 0..n {
                for v in (u + 1)..n {
                    if br[u] != br[v] && bd[u] == bd[v] {
                        gap = true;
                    }
                    if br[u] == br[v] && bd[u] != bd[v] {
                        unsound += 1;
                    }
                }
            }
            if gap {
                incomplete += 1;
            }
        }
    }

    assert_eq!(unsound, 0, "~R must never merge more than ~D (§6.1.2)");
    assert_eq!(models, 115, "n=3, 1 agent: 23 relations x 5 colourings");
    assert_eq!(incomplete, 6, "§6.2 measured 6/115 = 5.22%");
}
