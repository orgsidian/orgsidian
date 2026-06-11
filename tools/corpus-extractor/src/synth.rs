//! Deterministic synthesis (AC3): medium/large composition + edge-bucket
//! transforms. `.el` snippets are nearly all <1KB, so medium/large subset
//! members are *composed* from harvested snippets under generated `* Section k`
//! headings; edge-case members are mechanical transforms (CRLF re-encoding,
//! Unicode/RTL salting, malformed-but-valid mutations). No hand-authored org
//! text — harvested material plus mechanical transforms only (the one
//! documented exception: the Unicode salt line used iff the harvest yields no
//! Unicode/RTL content at all; ADR 0001).

use crate::model::Snippet;

/// Fixed literal seed for the composition RNG (determinism is a hard
/// requirement: same pin + same extractor code => byte-identical outputs).
pub const SYNTH_SEED: u64 = 0x6F72_6773_5F32_3035; // "orgs_205"

/// Salt line used ONLY when no harvested snippet carries Unicode/RTL content.
pub const UNICODE_SALT_LINE: &str = "* Unicode salt — مرحبا بالعالم שלום עולם こんにちは世界\n";

/// xorshift64* — tiny, dependency-free, deterministic PRNG.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // xorshift state must be non-zero.
        Rng(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform-ish index in `0..n` (n must be > 0).
    pub fn pick(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Compose a file of at least `target_bytes` (stopping right after crossing
/// the target) by joining snippets under generated `* Section k` headings.
/// `forced` snippets are always included first (construct-coverage packs /
/// edge salting); the remainder is a seeded selection from `pool`. Returns
/// the composed content and the source snippet ids in composition order.
pub fn compose(
    pool: &[Snippet],
    forced: &[&Snippet],
    target_bytes: usize,
    rng: &mut Rng,
) -> (String, Vec<String>) {
    let mut content = String::with_capacity(target_bytes + 1024);
    let mut sources = Vec::new();
    let mut section = 1usize;

    let push_part = |content: &mut String, sources: &mut Vec<String>, s: &Snippet, k: usize| {
        content.push_str(&format!("* Section {k}\n"));
        content.push_str(&s.content);
        if !content.ends_with('\n') {
            content.push('\n');
        }
        sources.push(s.id.clone());
    };

    for snippet in forced {
        push_part(&mut content, &mut sources, snippet, section);
        section += 1;
    }
    while content.len() < target_bytes && !pool.is_empty() {
        let snippet = &pool[rng.pick(pool.len())];
        push_part(&mut content, &mut sources, snippet, section);
        section += 1;
    }
    (content, sources)
}

/// Re-encode every LF as CRLF (idempotent on already-CRLF text).
pub fn to_crlf(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\n', "\r\n")
}

/// Mixed line endings: odd lines keep LF, even lines get CRLF.
pub fn to_mixed_eol(s: &str) -> String {
    let normalized = s.replace("\r\n", "\n");
    let mut out = String::with_capacity(normalized.len() + 64);
    for (i, line) in normalized.split_inclusive('\n').enumerate() {
        match line.strip_suffix('\n') {
            Some(body) if i % 2 == 1 => {
                out.push_str(body);
                out.push_str("\r\n");
            }
            _ => out.push_str(line),
        }
    }
    out
}

/// Malformed-but-valid: over-indent drawer-ish lines (`:NAME:` / `:END:` /
/// property keys) by 10 spaces — the same shape as the interim corpus file
/// `14_overindented_drawer.org`.
pub fn overindent_drawers(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 256);
    for line in s.split_inclusive('\n') {
        if line.trim_start().starts_with(':') {
            out.push_str("          ");
        }
        out.push_str(line);
    }
    out
}

/// Malformed-but-valid: trailing whitespace appended to every headline line.
pub fn trailing_ws_headlines(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 256);
    for line in s.split_inclusive('\n') {
        match line.strip_suffix('\n') {
            Some(body) if body.starts_with('*') && body.contains(' ') => {
                out.push_str(body);
                out.push_str("   \n");
            }
            _ => out.push_str(line),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Snippet;
    use std::collections::BTreeSet;

    fn snippet(id: &str, content: &str) -> Snippet {
        Snippet {
            id: id.to_string(),
            deftest: format!("test-org-element/{id}"),
            content: content.to_string(),
            constructs: BTreeSet::new(),
        }
    }

    #[test]
    fn rng_is_deterministic() {
        let a: Vec<u64> = {
            let mut r = Rng::new(SYNTH_SEED);
            (0..8).map(|_| r.next_u64()).collect()
        };
        let b: Vec<u64> = {
            let mut r = Rng::new(SYNTH_SEED);
            (0..8).map(|_| r.next_u64()).collect()
        };
        assert_eq!(a, b);
    }

    #[test]
    fn compose_is_deterministic_and_reaches_target() {
        let pool: Vec<Snippet> = (0..10)
            .map(|i| snippet(&format!("s{i}"), &format!("* H{i}\nbody {i}\n")))
            .collect();
        let forced = [&pool[3]];
        let (c1, s1) = compose(&pool, &forced, 500, &mut Rng::new(SYNTH_SEED));
        let (c2, s2) = compose(&pool, &forced, 500, &mut Rng::new(SYNTH_SEED));
        assert_eq!(c1, c2, "same seed => byte-identical composition");
        assert_eq!(s1, s2);
        assert!(c1.len() >= 500);
        assert_eq!(s1[0], "s3", "forced snippet comes first");
        assert!(c1.starts_with("* Section 1\n* H3\n"));
    }

    #[test]
    fn compose_records_all_sources_in_order() {
        let pool: Vec<Snippet> = (0..3).map(|i| snippet(&format!("s{i}"), "x\n")).collect();
        let (content, sources) = compose(&pool, &[], 40, &mut Rng::new(7));
        assert_eq!(content.matches("* Section ").count(), sources.len());
    }

    #[test]
    fn crlf_transform_re_encodes_every_line() {
        let crlf = to_crlf("* H\nbody\n");
        assert_eq!(crlf, "* H\r\nbody\r\n");
        assert_eq!(to_crlf(&crlf), crlf, "idempotent");
    }

    #[test]
    fn mixed_eol_alternates() {
        let mixed = to_mixed_eol("a\nb\nc\nd\n");
        assert_eq!(mixed, "a\nb\r\nc\nd\r\n");
        assert!(mixed.contains("\r\n") && mixed.contains("b\r\n"));
    }

    #[test]
    fn overindent_hits_drawer_lines_only() {
        let out = overindent_drawers("* H\n:PROPERTIES:\n:ID: x\n:END:\nbody\n");
        assert_eq!(
            out,
            "* H\n          :PROPERTIES:\n          :ID: x\n          :END:\nbody\n"
        );
    }

    #[test]
    fn trailing_ws_hits_headlines_only() {
        let out = trailing_ws_headlines("* Head one\nbody\n** Head two\n");
        assert_eq!(out, "* Head one   \nbody\n** Head two   \n");
    }
}
