//! Scoring — the whole point of the contract, and deliberately pure.
//!
//! No host calls here, so this module compiles and unit-tests natively. That
//! matters more than usual: the band is the only thing that ever leaves the
//! enclave, so its derivation is the part that most needs to be verifiable.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// One statement line. Amounts are signed minor units: credits positive,
/// debits negative.
#[derive(Debug, Clone, Deserialize)]
pub struct Txn {
    pub amount_minor: i64,
    /// Days before "now". 0 = today. Used only for bucketing into months.
    pub age_days: u32,
    #[serde(default)]
    pub balance_after_minor: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct Statement {
    pub currency: String,
    pub transactions: Vec<Txn>,
}

/// What the lender receives. Note what is *absent*: no transactions, no
/// balances, no counterparties, no dates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Band {
    /// `A` (strongest) … `D` (weakest), or `U` when there is too little history.
    pub band: char,
    /// 0..=100, monotone with band. Present so a lender can rank within a band.
    pub score: u8,
    /// Months of history the score is based on.
    pub months_observed: u32,
    /// Coarse inflow bucket, order of magnitude only — never the exact figure.
    pub inflow_bucket: &'static str,
    /// Machine-readable reasons, for adverse-action notices.
    pub reasons: Vec<String>,
}

/// Aggregates computed in-enclave and then discarded.
#[derive(Debug, Clone, PartialEq)]
pub struct Aggregates {
    pub months_observed: u32,
    pub median_monthly_inflow_minor: i64,
    pub inflow_volatility_pct: u32,
    pub negative_balance_days: u32,
    pub debit_to_credit_pct: u32,
}

const MIN_MONTHS: u32 = 3;

pub fn parse_statement(bytes: &[u8]) -> Result<Statement, String> {
    let s: Statement =
        serde_json::from_slice(bytes).map_err(|e| format!("statement: bad JSON: {e}"))?;
    if s.currency.len() != 3 || !s.currency.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(format!(
            "statement: currency must be a 3-letter uppercase ISO-4217 code, got {:?}",
            s.currency
        ));
    }
    Ok(s)
}

/// Reduce a statement to the handful of numbers the score depends on.
pub fn aggregate(stmt: &Statement) -> Aggregates {
    // bucket by month (30-day buckets — good enough, and avoids a calendar dep)
    let mut monthly_inflow: Vec<i64> = Vec::new();
    let mut max_month = 0u32;
    for t in &stmt.transactions {
        let m = t.age_days / 30;
        max_month = max_month.max(m);
        while monthly_inflow.len() <= m as usize {
            monthly_inflow.push(0);
        }
        if t.amount_minor > 0 {
            monthly_inflow[m as usize] += t.amount_minor;
        }
    }

    let months_observed = if stmt.transactions.is_empty() { 0 } else { max_month + 1 };

    let mut sorted = monthly_inflow.clone();
    sorted.sort_unstable();
    let median = if sorted.is_empty() {
        0
    } else if sorted.len() % 2 == 1 {
        sorted[sorted.len() / 2]
    } else {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2
    };

    // volatility as mean absolute deviation from the median, in percent of median
    let volatility_pct = if median > 0 && !monthly_inflow.is_empty() {
        let total_dev: i64 = monthly_inflow.iter().map(|v| (v - median).abs()).sum();
        let mad = total_dev / monthly_inflow.len() as i64;
        ((mad.saturating_mul(100)) / median).min(1000) as u32
    } else {
        0
    };

    let credits: i64 = stmt.transactions.iter().filter(|t| t.amount_minor > 0).map(|t| t.amount_minor).sum();
    let debits: i64 = stmt.transactions.iter().filter(|t| t.amount_minor < 0).map(|t| -t.amount_minor).sum();
    let debit_to_credit_pct = if credits > 0 {
        ((debits.saturating_mul(100)) / credits).min(1000) as u32
    } else if debits > 0 {
        1000
    } else {
        0
    };

    let negative_balance_days = stmt
        .transactions
        .iter()
        .filter(|t| t.balance_after_minor.is_some_and(|b| b < 0))
        .count() as u32;

    Aggregates {
        months_observed,
        median_monthly_inflow_minor: median,
        inflow_volatility_pct: volatility_pct,
        negative_balance_days,
        debit_to_credit_pct,
    }
}

/// Order-of-magnitude bucket. Coarse on purpose: a lender needs scale, not the
/// borrower's actual salary.
pub fn inflow_bucket(median_minor: i64) -> &'static str {
    match median_minor {
        i64::MIN..=0 => "none",
        1..=49_999_99 => "under_50k",
        5_000_000..=19_999_999 => "50k_200k",
        20_000_000..=99_999_999 => "200k_1m",
        _ => "over_1m",
    }
}

/// Map aggregates to a band. Deterministic, and every deduction is explained.
pub fn score(agg: &Aggregates) -> Band {
    let mut reasons: Vec<String> = Vec::new();

    if agg.months_observed < MIN_MONTHS {
        return Band {
            band: 'U',
            score: 0,
            months_observed: agg.months_observed,
            inflow_bucket: inflow_bucket(agg.median_monthly_inflow_minor),
            reasons: alloc::vec![format!(
                "insufficient_history: {} of {} months required",
                agg.months_observed, MIN_MONTHS
            )],
        };
    }

    let mut points: i32 = 100;

    if agg.median_monthly_inflow_minor <= 0 {
        points -= 45;
        reasons.push("no_recorded_inflow".to_string());
    }

    // Volatility: steady income is the strongest positive signal.
    match agg.inflow_volatility_pct {
        0..=20 => {}
        21..=45 => {
            points -= 10;
            reasons.push("moderate_income_volatility".to_string());
        }
        46..=80 => {
            points -= 22;
            reasons.push("high_income_volatility".to_string());
        }
        _ => {
            points -= 32;
            reasons.push("very_high_income_volatility".to_string());
        }
    }

    // Spending against income.
    match agg.debit_to_credit_pct {
        0..=70 => {}
        71..=90 => {
            points -= 8;
            reasons.push("elevated_outflow_ratio".to_string());
        }
        91..=105 => {
            points -= 18;
            reasons.push("outflow_matches_inflow".to_string());
        }
        _ => {
            points -= 28;
            reasons.push("outflow_exceeds_inflow".to_string());
        }
    }

    // Overdrafts.
    match agg.negative_balance_days {
        0 => {}
        1..=2 => {
            points -= 8;
            reasons.push("occasional_negative_balance".to_string());
        }
        3..=6 => {
            points -= 18;
            reasons.push("frequent_negative_balance".to_string());
        }
        _ => {
            points -= 30;
            reasons.push("persistent_negative_balance".to_string());
        }
    }

    // Reward a long track record, but never above the cap.
    if agg.months_observed >= 12 {
        points += 5;
        reasons.push("twelve_months_history".to_string());
    }

    let score = points.clamp(0, 100) as u8;
    let band = match score {
        85..=100 => 'A',
        70..=84 => 'B',
        50..=69 => 'C',
        _ => 'D',
    };

    if reasons.is_empty() {
        reasons.push("no_adverse_signals".to_string());
    }

    Band {
        band,
        score,
        months_observed: agg.months_observed,
        inflow_bucket: inflow_bucket(agg.median_monthly_inflow_minor),
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stmt(txns: Vec<Txn>) -> Statement {
        Statement { currency: "NGN".to_string(), transactions: txns }
    }

    fn t(amount: i64, age_days: u32) -> Txn {
        Txn { amount_minor: amount, age_days, balance_after_minor: None }
    }

    /// Steady salary, modest spending, no overdrafts, a year of history.
    fn steady_earner() -> Statement {
        let mut v = Vec::new();
        for m in 0..12 {
            v.push(t(30_000_00, m * 30 + 1));
            v.push(t(-15_000_00, m * 30 + 10));
        }
        stmt(v)
    }

    #[test]
    fn thin_file_is_unscoreable() {
        let b = score(&aggregate(&stmt(alloc::vec![t(10_000_00, 5)])));
        assert_eq!(b.band, 'U');
        assert_eq!(b.score, 0);
        assert!(b.reasons[0].contains("insufficient_history"));
    }

    #[test]
    fn steady_earner_scores_top_band() {
        let b = score(&aggregate(&steady_earner()));
        assert_eq!(b.band, 'A', "got {b:?}");
        assert_eq!(b.months_observed, 12);
    }

    #[test]
    fn volatile_income_scores_lower_than_steady() {
        let steady = score(&aggregate(&steady_earner()));
        let mut v = Vec::new();
        for m in 0..12 {
            let amount = if m % 3 == 0 { 90_000_00 } else { 2_000_00 };
            v.push(t(amount, m * 30 + 1));
            v.push(t(-15_000_00, m * 30 + 10));
        }
        let volatile = score(&aggregate(&stmt(v)));
        assert!(
            volatile.score < steady.score,
            "volatile {} should score below steady {}",
            volatile.score,
            steady.score
        );
    }

    #[test]
    fn overdrafts_are_penalised_and_explained() {
        let mut v = Vec::new();
        for m in 0..6 {
            v.push(t(20_000_00, m * 30 + 1));
            v.push(Txn { amount_minor: -25_000_00, age_days: m * 30 + 15, balance_after_minor: Some(-5_000_00) });
        }
        let b = score(&aggregate(&stmt(v)));
        assert!(b.reasons.iter().any(|r| r.contains("negative_balance")), "{b:?}");
        assert!(b.reasons.iter().any(|r| r.contains("outflow")), "{b:?}");
        assert!(b.score < 70, "expected weak score, got {}", b.score);
    }

    #[test]
    fn score_is_deterministic() {
        let a = score(&aggregate(&steady_earner()));
        let b = score(&aggregate(&steady_earner()));
        assert_eq!(a, b);
    }

    #[test]
    fn buckets_are_coarse() {
        // amounts are minor units (kobo); labels are naira thresholds
        assert_eq!(inflow_bucket(0), "none");
        assert_eq!(inflow_bucket(-5_000), "none");
        assert_eq!(inflow_bucket(1_000_00), "under_50k"); // ₦1,000
        assert_eq!(inflow_bucket(30_000_00), "under_50k"); // ₦30,000
        assert_eq!(inflow_bucket(4_999_999), "under_50k"); // just under ₦50,000
        assert_eq!(inflow_bucket(5_000_000), "50k_200k"); // exactly ₦50,000
        assert_eq!(inflow_bucket(15_000_000), "50k_200k"); // ₦150,000
        assert_eq!(inflow_bucket(20_000_000), "200k_1m"); // exactly ₦200,000
        assert_eq!(inflow_bucket(50_000_000), "200k_1m"); // ₦500,000
        assert_eq!(inflow_bucket(100_000_000), "over_1m"); // ₦1,000,000
    }

    /// A strictly better borrower must never receive a worse band.
    #[test]
    fn better_aggregates_never_score_worse() {
        let base = Aggregates {
            months_observed: 12,
            median_monthly_inflow_minor: 30_000_00,
            inflow_volatility_pct: 60,
            negative_balance_days: 5,
            debit_to_credit_pct: 95,
        };
        let improved = Aggregates {
            inflow_volatility_pct: 10,
            negative_balance_days: 0,
            debit_to_credit_pct: 40,
            ..base.clone()
        };
        let a = score(&base);
        let b = score(&improved);
        assert!(
            b.score > a.score,
            "improving every signal should raise the score: {} -> {}",
            a.score,
            b.score
        );
        let rank = |c: char| match c {
            'A' => 4,
            'B' => 3,
            'C' => 2,
            'D' => 1,
            _ => 0,
        };
        assert!(rank(b.band) >= rank(a.band), "{:?} vs {:?}", a.band, b.band);
    }

    #[test]
    fn band_boundaries_match_score_ranges() {
        // drive real aggregates across the range and check band/score agree
        for agg in [
            Aggregates { months_observed: 12, median_monthly_inflow_minor: 30_000_00, inflow_volatility_pct: 0,  negative_balance_days: 0,  debit_to_credit_pct: 0 },
            Aggregates { months_observed: 6,  median_monthly_inflow_minor: 30_000_00, inflow_volatility_pct: 30, negative_balance_days: 1,  debit_to_credit_pct: 80 },
            Aggregates { months_observed: 6,  median_monthly_inflow_minor: 30_000_00, inflow_volatility_pct: 60, negative_balance_days: 4,  debit_to_credit_pct: 95 },
            Aggregates { months_observed: 4,  median_monthly_inflow_minor: 0,         inflow_volatility_pct: 90, negative_balance_days: 10, debit_to_credit_pct: 200 },
        ] {
            let b = score(&agg);
            let expected = match b.score {
                85..=100 => 'A',
                70..=84 => 'B',
                50..=69 => 'C',
                _ => 'D',
            };
            assert_eq!(b.band, expected, "score {} should map to {expected}", b.score);
        }
    }

    /// The privacy property: the serialised band exposes no transaction data.
    #[test]
    fn serialised_band_leaks_no_transaction_detail() {
        let b = score(&aggregate(&steady_earner()));
        let json = serde_json::to_string(&b).unwrap();
        for forbidden in ["amount_minor", "balance_after", "transactions", "age_days"] {
            assert!(!json.contains(forbidden), "band JSON must not contain {forbidden}: {json}");
        }
        // and no exact figure — only the coarse bucket
        assert!(!json.contains("3000000"), "exact inflow must not appear: {json}");
    }

    #[test]
    fn rejects_bad_currency() {
        let err = parse_statement(br#"{"currency":"ngn","transactions":[]}"#).unwrap_err();
        assert!(err.contains("ISO-4217"), "{err}");
    }
}
