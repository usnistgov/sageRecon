//! Fisher exact test and Benjamini-Hochberg correction.
//!
//! Hand-rolled because the crate has no statistics dependency and this needs only
//! two functions. Both are pinned by test against SciPy
//! (`scipy.stats.fisher_exact(..., alternative="greater")` and
//! `scipy.stats.false_discovery_control`), so a drift in either is caught here and
//! not discovered in a report.
//!
//! Why the odds ratio and not the enrichment ratio: enrichment is
//! peak-fraction / run-fraction and so cannot exceed `1/background`. A residue at
//! 70% background caps near 1.4x however real the modification is. The odds ratio
//! carries no such cap. See NOTES "Step 2 decision rule — route by specificity".

/// Natural log of the gamma function (Lanczos approximation, g=7, n=9).
fn ln_gamma(x: f64) -> f64 {
    // ⚠ `clippy::excessive_precision` and `clippy::inconsistent_digit_grouping`
    // both fire on this table and are NOT applied. These are the published
    // Lanczos g=7, n=9 coefficients, transcribed as the source writes them. The
    // digit count and the grouping are the SOURCE's, not ours, so a reader can
    // compare them against it character by character. Truncating them parses to
    // the same f64 — which is exactly why the change would be all cost and no
    // benefit: it breaks the comparison and buys no accuracy. `ln_gamma` is
    // pinned by test against SciPy; if these digits ever need to move, that is a
    // deliberate recorded edit, not a lint fix.
    #[allow(clippy::excessive_precision, clippy::inconsistent_digit_grouping)]
    const C: [f64; 9] = [
        0.999_999_999_999_809_93,
        676.520_368_121_885_1,
        -1259.139_216_722_402_8,
        771.323_428_777_653_13,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // Reflection, so the approximation is only ever used on the right half.
        std::f64::consts::PI.ln() - (std::f64::consts::PI * x).sin().ln() - ln_gamma(1.0 - x)
    } else {
        let x = x - 1.0;
        let mut a = C[0];
        let t = x + 7.5;
        for (i, c) in C.iter().enumerate().skip(1) {
            a += c / (x + i as f64);
        }
        0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
    }
}

fn ln_binom(n: u64, k: u64) -> f64 {
    if k > n {
        return f64::NEG_INFINITY;
    }
    ln_gamma(n as f64 + 1.0) - ln_gamma(k as f64 + 1.0) - ln_gamma((n - k) as f64 + 1.0)
}

/// Probability of exactly `k` successes under the hypergeometric distribution.
fn hypergeom_pmf(k: u64, total: u64, successes: u64, draws: u64) -> f64 {
    if successes > total || draws > total || k > draws || k > successes {
        return 0.0;
    }
    if (draws - k) > (total - successes) {
        return 0.0;
    }
    (ln_binom(successes, k) + ln_binom(total - successes, draws - k) - ln_binom(total, draws)).exp()
}

/// One-sided (greater) Fisher exact test on `[[a, b], [c, d]]`.
///
/// Returns `(odds_ratio, p_value)`. The odds ratio is the conditional-free sample
/// odds ratio `(a*d)/(b*c)`, which is `f64::INFINITY` when `b` or `c` is zero —
/// callers that need a finite value apply a Haldane-Anscombe correction themselves.
pub fn fisher_exact_greater(a: u64, b: u64, c: u64, d: u64) -> (f64, f64) {
    let total = a + b + c + d;
    let row1 = a + b;
    let col1 = a + c;
    let odds = if b == 0 || c == 0 {
        f64::INFINITY
    } else {
        (a as f64 * d as f64) / (b as f64 * c as f64)
    };
    if total == 0 {
        return (odds, 1.0);
    }
    // Sum the upper tail: P(X >= a), X ~ Hypergeometric(total, col1, row1).
    let hi = row1.min(col1);
    let mut p = 0.0;
    for k in a..=hi {
        p += hypergeom_pmf(k, total, col1, row1);
    }
    (odds, p.clamp(0.0, 1.0))
}

/// Benjamini-Hochberg adjusted p-values, returned in the input order.
///
/// Matches `scipy.stats.false_discovery_control(p, method="bh")`: adjusted values
/// are enforced monotone non-decreasing from the largest p downward, then clamped
/// to 1.
pub fn benjamini_hochberg(p: &[f64]) -> Vec<f64> {
    let n = p.len();
    if n == 0 {
        return Vec::new();
    }
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|&i, &j| p[i].partial_cmp(&p[j]).unwrap_or(std::cmp::Ordering::Equal));
    let mut adj = vec![0.0_f64; n];
    let mut running = f64::INFINITY;
    for rank in (1..=n).rev() {
        let i = idx[rank - 1];
        let scaled = p[i] * n as f64 / rank as f64;
        running = running.min(scaled);
        adj[i] = running.min(1.0);
    }
    adj
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every expected value here came from SciPy. If these drift, the report's
    /// statistics have drifted with them.
    #[test]
    fn fisher_matches_scipy() {
        let (o, p) = fisher_exact_greater(8, 2, 1, 5);
        assert!((o - 20.0).abs() < 1e-9, "odds {o}");
        assert!((p - 0.024_475_524_475_524_483).abs() < 1e-12, "p {p}");

        let (o, p) = fisher_exact_greater(10, 0, 3, 7);
        assert!(o.is_infinite(), "odds {o}");
        assert!((p - 0.001_547_987_616_099_071).abs() < 1e-12, "p {p}");

        let (o, p) = fisher_exact_greater(5, 5, 5, 5);
        assert!((o - 1.0).abs() < 1e-9, "odds {o}");
        assert!((p - 0.671_859_100_651_670_3).abs() < 1e-12, "p {p}");

        // The real bcell +57 table: 2845 of 2969 band PSMs contain Cys.
        let (o, p) = fisher_exact_greater(2845, 124, 2479, 49971);
        assert!((o - 462.489_736_366_120_55).abs() < 1e-6, "odds {o}");
        assert!(p < 1e-300, "p {p}");
    }

    #[test]
    fn bh_matches_scipy() {
        let p = [
            0.001, 0.008, 0.039, 0.041, 0.042, 0.06, 0.074, 0.205, 0.212, 0.216,
        ];
        let want = [
            0.01,
            0.04,
            0.084,
            0.084,
            0.084,
            0.1,
            0.105_714_285_7,
            0.216,
            0.216,
            0.216,
        ];
        let got = benjamini_hochberg(&p);
        for (g, w) in got.iter().zip(want.iter()) {
            assert!((g - w).abs() < 1e-9, "got {g} want {w}");
        }
    }

    #[test]
    fn bh_handles_edges() {
        assert!(benjamini_hochberg(&[]).is_empty());
        let one = benjamini_hochberg(&[0.5]);
        assert!((one[0] - 0.5).abs() < 1e-12);
        // Adjusted values never exceed 1.
        for v in benjamini_hochberg(&[0.9, 0.95, 0.99]) {
            assert!(v <= 1.0 + 1e-12, "{v}");
        }
    }
}
