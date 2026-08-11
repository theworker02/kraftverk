# Statistics

## Per-sample summary

For a numeric vector: `n`, mean, median, min, max, variance, stddev, coefficient of variation (CoV), percentiles (p10/p25/p75/p90/p95), approximate mean CI, IQR outlier count.

## Comparison classes

Comparing candidate vs baseline (higher-is-better Kraft Index samples):

| Class | Meaning |
|-------|---------|
| `CONFIRMED_IMPROVEMENT` | CIs separated; ≥ ~3% and candidate CI above baseline |
| `LIKELY_IMPROVEMENT` | Positive shift without full confirmation |
| `NO_SIGNIFICANT_CHANGE` | Overlap / below indifference threshold |
| `LIKELY_REGRESSION` | Negative shift without full confirmation |
| `CONFIRMED_REGRESSION` | CIs separated; ≤ ~-3% |
| `UNSTABLE_RESULT` | High CoV or too few samples |

Defaults: 95% CI (z≈1.96), indifference 0.5%, outlier IQR k=1.5.

## Honesty

Small N and non-normal benches mean classifications are **decision aids**, not formal hypothesis tests. M1 prioritizes transparency over pretending lab-grade stats.
