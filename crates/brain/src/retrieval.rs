//! Retrieval scoring primitives — pure functions, no I/O.
//!
//! Mirrors the Cerebras design: no single scorer is trusted on its own.
//! Full-text, semantic, and recency signals each produce a ranked list, and
//! the lists are fused with reciprocal rank fusion (RRF) at query time.

/// Cosine similarity between two vectors. Zero for empty/zero-norm inputs.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += (*x as f64) * (*y as f64);
        na += (*x as f64) * (*x as f64);
        nb += (*y as f64) * (*y as f64);
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    (dot / (na.sqrt() * nb.sqrt())) as f32
}

/// Exponential decay of relevance with age: `0.5 ^ (age / half_life)`.
/// Slack answers expire — two threads can answer the same question and the
/// six-month-old one may describe infrastructure that no longer exists.
pub fn recency_weight(age_seconds: f64, half_life_days: f64) -> f64 {
    let half_life_secs = half_life_days * 86_400.0;
    if half_life_secs <= 0.0 {
        return 1.0;
    }
    let age = age_seconds.max(0.0);
    (0.5f64).powf(age / half_life_secs)
}

/// Reciprocal rank fusion: `score += 1.0 / (k + rank)` for each list a doc
/// appears in. The smoothing constant `k` makes consensus matter more than a
/// single strong vote — a doc near the top of several lists beats one that
/// ranks first in only one (Cerebras, following Cormack et al. SIGIR 2009).
pub fn rrf_merge(lists: &[Vec<(u64, f64)>], k: f64) -> Vec<(u64, f64)> {
    let mut acc: std::collections::HashMap<u64, f64> = std::collections::HashMap::new();
    for list in lists {
        for (rank, (id, _)) in list.iter().enumerate() {
            let entry = acc.entry(*id).or_insert(0.0);
            *entry += 1.0 / (k + (rank as f64) + 1.0);
        }
    }
    let mut out: Vec<(u64, f64)> = acc.into_iter().collect();
    out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// Normalize scores into `[0, 1]` by max. Empty input stays empty.
pub fn normalize(mut scored: Vec<(u64, f64)>) -> Vec<(u64, f64)> {
    let max = scored.iter().fold(0.0f64, |acc, (_, s)| acc.max(*s));
    if max > 0.0 {
        for (_, s) in scored.iter_mut() {
            *s /= max;
        }
    }
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_is_one() {
        let v = [0.1, 0.2, 0.3, 0.4];
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_is_zero() {
        let a = [1.0, 0.0];
        let b = [0.0, 1.0];
        assert!((cosine(&a, &b)).abs() < 1e-6);
    }

    #[test]
    fn cosine_mismatched_dims_is_zero() {
        assert_eq!(cosine(&[1.0, 2.0], &[1.0]), 0.0);
    }

    #[test]
    fn recency_halves_per_half_life() {
        let w = recency_weight(86_400.0, 1.0);
        assert!((w - 0.5).abs() < 1e-9);
        let fresh = recency_weight(0.0, 1.0);
        assert_eq!(fresh, 1.0);
        let old = recency_weight(7.0 * 86_400.0, 1.0);
        assert!((old - 0.5f64.powi(7)).abs() < 1e-9);
    }

    #[test]
    fn rrf_consensus_beats_single_top_rank() {
        // Doc 1 ranks #1 in one list. Doc 2 ranks #2 in three lists.
        let lists: Vec<Vec<(u64, f64)>> = vec![
            vec![(1, 1.0), (2, 0.5)],
            vec![(2, 0.5), (1, 0.4)],
            vec![(2, 0.5), (1, 0.4)],
        ];
        let merged = rrf_merge(&lists, 60.0);
        assert_eq!(merged[0].0, 2, "consensus should beat a single top vote");
    }

    #[test]
    fn rrf_respects_rank_within_list() {
        let lists: Vec<Vec<(u64, f64)>> = vec![vec![(1, 1.0), (2, 0.5)], vec![(1, 1.0), (2, 0.5)]];
        let merged = rrf_merge(&lists, 60.0);
        assert_eq!(merged[0].0, 1);
    }

    #[test]
    fn normalize_scales_to_one() {
        let scored = vec![(1, 4.0), (2, 1.0)];
        let n = normalize(scored);
        assert!((n[0].1 - 1.0).abs() < 1e-9);
        assert!((n[1].1 - 0.25).abs() < 1e-9);
    }
}
