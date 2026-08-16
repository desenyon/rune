//! Offline semantic retrieval via hashed character n-grams.
//!
//! This is a local embedding provider, not a neural model. Remote/local LLM
//! embedders remain pluggable through `rune-semantic`. Structural search still
//! works when this path is unused.

const DIM: usize = 256;

/// Hash-embed `text` into a unit-normalized 256-dimension vector.
pub fn hash_embed(text: &str) -> Vec<f32> {
    let mut vec = vec![0f32; DIM];
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    if bytes.len() < 3 {
        for b in bytes {
            let idx = (*b as usize) % DIM;
            vec[idx] += 1.0;
        }
        return normalize(&mut vec);
    }
    for window in bytes.windows(3) {
        let h = hash3(window[0], window[1], window[2]);
        vec[(h as usize) % DIM] += 1.0;
        vec[((h >> 8) as usize) % DIM] += 0.5;
    }
    for token in lower.split(|c: char| !c.is_ascii_alphanumeric()) {
        if token.len() < 2 {
            continue;
        }
        let h = hash_bytes(token.as_bytes());
        vec[(h as usize) % DIM] += 2.0;
    }
    normalize(&mut vec)
}

pub fn cosine(a: &[f32], b: &[f32]) -> f64 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += f64::from(*x) * f64::from(*y);
    }
    dot.clamp(0.0, 1.0)
}

fn normalize(vec: &mut [f32]) -> Vec<f32> {
    let mut sum = 0.0f32;
    for v in vec.iter() {
        sum += *v * *v;
    }
    let norm = sum.sqrt();
    if norm > 0.0 {
        for v in vec.iter_mut() {
            *v /= norm;
        }
    }
    vec.to_vec()
}

fn hash3(a: u8, b: u8, c: u8) -> u32 {
    let mut h = 0x811c9dc5u32;
    h ^= u32::from(a);
    h = h.wrapping_mul(0x01000193);
    h ^= u32::from(b);
    h = h.wrapping_mul(0x01000193);
    h ^= u32::from(c);
    h.wrapping_mul(0x01000193)
}

fn hash_bytes(bytes: &[u8]) -> u32 {
    let mut h = 0x811c9dc5u32;
    for b in bytes {
        h ^= u32::from(*b);
        h = h.wrapping_mul(0x01000193);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similar_statements_outrank_unrelated() {
        let q = hash_embed("authentication token rotation");
        let close = hash_embed("rotate authentication tokens in the session store");
        let far = hash_embed("render terminal widgets with gold accents");
        assert!(cosine(&q, &close) > cosine(&q, &far));
    }
}
