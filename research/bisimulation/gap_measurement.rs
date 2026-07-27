// TIER 1: measure the incompleteness gap.
//
//   ~R  = bisimilarity over {R_i, R_i^-1}          <- what [T]/[J] compute
//   ~D  = bisimilarity over {~_i, Bel_i, R_i, C}   <- sec 6.3's proposal
//
// K/B/[]/C are boxes over exactly the ~D relations, so on finite models ~D IS modal
// equivalence (Hennessy-Milner). Therefore:
//   - ~R subset ~D  would mean [J] is sound but merges too little  -> incomplete
//   - any pair in ~D but not ~R is a concrete instance of [T]'s incompleteness
//
// Reports how often that happens and prints the smallest witness.

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
        if class >> w & 1 == 0 { continue; }
        let mut ok = true;
        for x in 0..n {
            if class >> x & 1 == 1 && (r[x] >> w & 1 == 0) { ok = false; break; }
        }
        if ok { m |= 1 << w; }
    }
    m
}

fn bel_rel(r: &Rel, n: usize) -> Rel {
    let mut b = [0u16; NMAX];
    for u in 0..n { b[u] = max_set(r, u, n); }
    b
}

fn union_closure(rels: &[Rel], n: usize) -> Rel {
    let mut c = [0u16; NMAX];
    for u in 0..n {
        for r in rels { c[u] |= r[u]; }
        c[u] |= 1 << u;
    }
    for _ in 0..n {
        for u in 0..n {
            let mut add = 0u16;
            for v in 0..n { if c[u] >> v & 1 == 1 { add |= c[v]; } }
            c[u] |= add;
        }
    }
    c
}

fn canonicalise(p: &[usize], n: usize) -> Vec<usize> {
    let mut map = vec![usize::MAX; n + 2];
    let mut next = 0;
    let mut out = vec![0usize; n];
    for u in 0..n {
        if map[p[u]] == usize::MAX { map[p[u]] = next; next += 1; }
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
                for v in 0..n { if rel[u] >> v & 1 == 1 { mask |= 1 << block[v]; } }
                sig.push(mask);
            }
            let key = (block[u], sig);
            next[u] = match sigs.iter().position(|s| *s == key) {
                Some(i) => i,
                None => { sigs.push(key); sigs.len() - 1 }
            };
        }
        let next = canonicalise(&next, n);
        if next == block { return block; }
        block = next;
    }
}

fn rels_R(ra: &[Rel], n: usize) -> Vec<Rel> {
    let mut v = Vec::new();
    for r in ra { v.push(*r); v.push(transpose(r, n)); }
    v
}

fn rels_D(ra: &[Rel], n: usize) -> Vec<Rel> {
    let mut v = Vec::new();
    let mut comps = Vec::new();
    for r in ra {
        v.push(*r);                       // []_i
        let c = comparability(r, n);
        v.push(c);                        // K_i
        comps.push(c);
        v.push(bel_rel(r, n));            // B_i
    }
    v.push(union_closure(&comps, n));     // C over all agents
    v
}

fn build(class: &[usize], level: &[usize], n: usize) -> Rel {
    let mut r = [0u16; NMAX];
    for u in 0..n {
        for v in 0..n {
            if class[u] == class[v] && level[u] <= level[v] { r[u] |= 1 << v; }
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
        for u in 0..n { class[u] = (x % n as u64) as usize; x /= n as u64; }
        for l in 0..total {
            let mut level = vec![0usize; n];
            let mut y = l;
            for u in 0..n { level[u] = (y % n as u64) as usize; y /= n as u64; }
            let r = build(&class, &level, n);
            if seen.insert(r) { out.push(r); }
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
        for u in 0..n { v[u] = (x % n as u64) as usize; x /= n as u64; }
        let v = canonicalise(&v, n);
        if seen.insert(v.clone()) { out.push(v); }
    }
    out
}

fn main() {
    for n in 2..=4usize {
        let rels = all_relations(n);
        let cols = colourings(n);
        for agents in 1..=2usize {
            let mut models: u64 = 0;
            let mut gap_models: u64 = 0;      // ~R separates a pair that ~D merges
            let mut unsound: u64 = 0;         // ~R merges a pair ~D separates (would be a BUG)
            let mut witness: Option<(Vec<Rel>, Vec<usize>)> = None;

            let combos: Vec<Vec<Rel>> = if agents == 1 {
                rels.iter().map(|r| vec![*r]).collect()
            } else {
                let mut v = Vec::new();
                for a in &rels { for b in &rels { v.push(vec![*a, *b]); } }
                v
            };

            for ra in &combos {
                for colour in &cols {
                    models += 1;
                    let br = coarsest_bisim(&rels_R(ra, n), colour, n);
                    let bd = coarsest_bisim(&rels_D(ra, n), colour, n);
                    let mut gap = false;
                    for u in 0..n {
                        for v in (u + 1)..n {
                            if br[u] != br[v] && bd[u] == bd[v] { gap = true; }
                            if br[u] == br[v] && bd[u] != bd[v] { unsound += 1; }
                        }
                    }
                    if gap {
                        gap_models += 1;
                        if witness.is_none() && n <= 3 {
                            witness = Some((ra.clone(), colour.clone()));
                        }
                    }
                }
            }
            println!(
                "n={} agents={}: {:>9} models | ~R incomplete on {:>8} ({:>5.2}%) | ~R unsound on {}",
                n, agents, models, gap_models,
                100.0 * gap_models as f64 / models as f64, unsound
            );
            if let Some((ra, colour)) = witness {
                let br = coarsest_bisim(&rels_R(&ra, n), &colour, n);
                let bd = coarsest_bisim(&rels_D(&ra, n), &colour, n);
                println!("    smallest witness:");
                for (i, r) in ra.iter().enumerate() {
                    print!("      R_{}: ", i);
                    for a in 0..n { for b in 0..n {
                        if a != b && r[a] >> b & 1 == 1 { print!("{}->{} ", a, b); }
                    }}
                    println!("(+refl)   Bel_{} = {:?}", i,
                        (0..n).map(|u| max_set(r, u, n)).collect::<Vec<_>>());
                }
                println!("      valuations: {:?}", colour);
                println!("      ~R blocks: {:?}   ~D blocks: {:?}", br, bd);
            }
        }
    }
}
