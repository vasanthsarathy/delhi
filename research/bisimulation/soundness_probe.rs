// Is bisimulation over {R_i, R_i^-1} SOUND for B_i in mB plausibility models?
//
// Exact criterion. B_i phi at u  <=>  phi at every w in ->_i^u.  Bisimilar worlds agree on all
// formulas iff, for every agent, their ->_i sets agree *up to bisimulation blocks*:
//
//        { block(w) : w in ->_i^u }  ==  { block(w) : w in ->_i^v }
//
// Comparing valuation sets instead is strictly weaker and misses nested-modality failures.
// If this fails for two worlds in the same block, contraction is UNSOUND.
//
// Structure fact used by the generator: a locally-well-preordered relation is exactly
// "partition the worlds into comparability classes, put a TOTAL PREORDER on each class."
// So (class[u], level[u]) with  u R v  <=>  class[u]==class[v] && level[u] <= level[v]
// enumerates them precisely.

use std::collections::HashSet;

const NMAX: usize = 8;
type Rel = [u16; NMAX];

fn comparability(r: &Rel, n: usize) -> Rel {
    let mut c = [0u16; NMAX];
    for u in 0..n {
        for v in 0..n {
            if r[u] >> v & 1 == 1 || r[v] >> u & 1 == 1 {
                c[u] |= 1 << v;
            }
        }
    }
    c
}

fn transpose(r: &Rel, n: usize) -> Rel {
    let mut t = [0u16; NMAX];
    for u in 0..n {
        for v in 0..n {
            if r[u] >> v & 1 == 1 {
                t[v] |= 1 << u;
            }
        }
    }
    t
}

fn max_set(r: &Rel, u: usize, n: usize) -> u16 {
    let c = comparability(r, n);
    let class = c[u];
    let mut m = 0u16;
    for w in 0..n {
        if class >> w & 1 == 0 {
            continue;
        }
        let mut ok = true;
        for x in 0..n {
            if class >> x & 1 == 1 && (r[x] >> w & 1 == 0) {
                ok = false;
                break;
            }
        }
        if ok {
            m |= 1 << w;
        }
    }
    m
}

fn canonicalise(p: &[usize], n: usize) -> Vec<usize> {
    let mut map = vec![usize::MAX; n + 2];
    let mut next = 0;
    let mut out = vec![0usize; n];
    for u in 0..n {
        if map[p[u]] == usize::MAX {
            map[p[u]] = next;
            next += 1;
        }
        out[u] = map[p[u]];
    }
    out
}

fn coarsest_bisim(rels: &[Rel], colour: &[usize], n: usize) -> Vec<usize> {
    let mut block = canonicalise(colour, n);
    loop {
        let mut sigs: Vec<(usize, Vec<u32>)> = Vec::new();
        let mut next = vec![0usize; n];
        for u in 0..n {
            let mut sig = Vec::with_capacity(rels.len());
            for rel in rels {
                let mut mask = 0u32;
                for v in 0..n {
                    if rel[u] >> v & 1 == 1 {
                        mask |= 1 << block[v];
                    }
                }
                sig.push(mask);
            }
            let key = (block[u], sig);
            next[u] = match sigs.iter().position(|s| *s == key) {
                Some(i) => i,
                None => {
                    sigs.push(key);
                    sigs.len() - 1
                }
            };
        }
        let next = canonicalise(&next, n);
        if next == block {
            return block;
        }
        block = next;
    }
}

// the EXACT criterion: block-set of ->_i^u
fn block_set(mask: u16, block: &[usize], n: usize) -> u32 {
    let mut s = 0u32;
    for w in 0..n {
        if mask >> w & 1 == 1 {
            s |= 1 << block[w];
        }
    }
    s
}

fn build(class: &[usize], level: &[usize], n: usize) -> Rel {
    let mut r = [0u16; NMAX];
    for u in 0..n {
        for v in 0..n {
            if class[u] == class[v] && level[u] <= level[v] {
                r[u] |= 1 << v;
            }
        }
    }
    r
}

fn all_relations(n: usize) -> Vec<Rel> {
    let mut seen: HashSet<Rel> = HashSet::new();
    let mut out = Vec::new();
    let total = (n as u64).pow(n as u32);
    for c in 0..total {
        let mut class = vec![0usize; n];
        let mut x = c;
        for u in 0..n {
            class[u] = (x % n as u64) as usize;
            x /= n as u64;
        }
        for l in 0..total {
            let mut level = vec![0usize; n];
            let mut y = l;
            for u in 0..n {
                level[u] = (y % n as u64) as usize;
                y /= n as u64;
            }
            let r = build(&class, &level, n);
            if seen.insert(r) {
                out.push(r);
            }
        }
    }
    out
}

fn colourings(n: usize) -> Vec<Vec<usize>> {
    let mut seen: HashSet<Vec<usize>> = HashSet::new();
    let mut out = Vec::new();
    let total = (n as u64).pow(n as u32);
    for c in 0..total {
        let mut v = vec![0usize; n];
        let mut x = c;
        for u in 0..n {
            v[u] = (x % n as u64) as usize;
            x /= n as u64;
        }
        let v = canonicalise(&v, n);
        if seen.insert(v.clone()) {
            out.push(v);
        }
    }
    out
}

fn check(rels_agent: &[Rel], colour: &[usize], n: usize) -> Option<(usize, usize, usize)> {
    let mut bisim_rels: Vec<Rel> = Vec::new();
    for r in rels_agent {
        bisim_rels.push(*r);
        bisim_rels.push(transpose(r, n));
    }
    let block = coarsest_bisim(&bisim_rels, colour, n);
    for u in 0..n {
        for v in (u + 1)..n {
            if block[u] != block[v] {
                continue;
            }
            for (ai, r) in rels_agent.iter().enumerate() {
                let bu = block_set(max_set(r, u, n), &block, n);
                let bv = block_set(max_set(r, v, n), &block, n);
                if bu != bv {
                    return Some((u, v, ai));
                }
            }
        }
    }
    None
}

fn report(rels: &[Rel], colour: &[usize], n: usize, u: usize, v: usize, ai: usize) {
    println!("  !!! COUNTEREXAMPLE (n={}) !!!", n);
    for (i, r) in rels.iter().enumerate() {
        print!("    R_{}: ", i);
        for a in 0..n {
            for b in 0..n {
                if a != b && r[a] >> b & 1 == 1 {
                    print!("{}->{} ", a, b);
                }
            }
        }
        println!("(+reflexive)");
    }
    println!("    valuation classes: {:?}", colour);
    println!("    worlds {} and {} bisimilar; agent {} max-sets differ", u, v, ai);
}

// xorshift, so the run is reproducible without a Date/rand dependency
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, k: usize) -> usize {
        (self.next() % k as u64) as usize
    }
}

fn main() {
    // ---------- exhaustive, n = 3 and 4 ----------
    for n in 3..=4usize {
        let rels = all_relations(n);
        let cols = colourings(n);
        println!(
            "n={}: {} locally-well-preordered relations, {} colourings",
            n,
            rels.len(),
            cols.len()
        );
        for agents in 1..=2usize {
            let mut found = None;
            let mut checked: u64 = 0;
            'outer: for i in 0..rels.len() {
                let combos: Vec<Vec<Rel>> = if agents == 1 {
                    vec![vec![rels[i]]]
                } else {
                    (0..rels.len()).map(|j| vec![rels[i], rels[j]]).collect()
                };
                for ra in combos {
                    for colour in &cols {
                        checked += 1;
                        if let Some((u, v, ai)) = check(&ra, colour, n) {
                            found = Some((ra.clone(), colour.clone(), u, v, ai));
                            break 'outer;
                        }
                    }
                }
            }
            match found {
                Some((ra, c, u, v, ai)) => report(&ra, &c, n, u, v, ai),
                None => println!("  {} agent(s), {} models: SOUND (no counterexample)", agents, checked),
            }
        }
    }

    // ---------- randomised, larger n ----------
    let mut rng = Rng(0x2545F4914F6CDD1D);
    for n in 5..=8usize {
        for agents in 2..=3usize {
            let trials = 3_000_000u64;
            let mut found = None;
            for _ in 0..trials {
                let ra: Vec<Rel> = (0..agents)
                    .map(|_| {
                        let class: Vec<usize> = (0..n).map(|_| rng.below(n)).collect();
                        let level: Vec<usize> = (0..n).map(|_| rng.below(n)).collect();
                        build(&class, &level, n)
                    })
                    .collect();
                let colour: Vec<usize> = (0..n).map(|_| rng.below(n)).collect();
                if let Some((u, v, ai)) = check(&ra, &colour, n) {
                    found = Some((ra, colour, u, v, ai));
                    break;
                }
            }
            match found {
                Some((ra, c, u, v, ai)) => report(&ra, &c, n, u, v, ai),
                None => println!(
                    "n={}, {} agents, {} random models: SOUND (no counterexample)",
                    n, agents, trials
                ),
            }
        }
    }
}
