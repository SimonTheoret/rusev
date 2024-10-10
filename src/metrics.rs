use ndarray::parallel::prelude::*;
use ndarray::prelude::*;
use ndarray::Array;
use num::Float as NumFloat;

struct PerClassScore(Vec<f32>, Vec<f32>, Vec<f32>, Vec<i32>);
struct AverageScore(f32, f32, f32, i32);

enum Scores {
    PerClassScore(PerClassScore),
    AverageScore(AverageScore),
}

#[derive(Debug)]
enum DivisionByZeroResultStrategy {
    ReplaceBy0,
    ReplaceBy1,
    Error,
    Unchecked,
}
impl Default for DivisionByZeroResultStrategy {
    fn default() -> Self {
        Self::ReplaceBy0
    }
}

enum Modifier {
    Predicted,
    True,
}

enum Average {
    None,
    Micro,
    Macro,
    Weighted,
}

trait Float: NumFloat + Send + Sync {
    fn zero() -> Self;
    fn one() -> Self;
}

impl Float for f32 {
    fn one() -> Self {
        1.0
    }
    fn zero() -> Self {
        0.0
    }
}

impl Float for f64 {
    fn one() -> Self {
        1.0
    }
    fn zero() -> Self {
        0.0
    }
}

// TODO: Change the warn_for `vec` for a slice (if possible)
fn prf_divide<F: Float, D: Dimension>(
    numerator: Array<F, D>,
    mut denominator: ArrayViewMut<F, D>,
    parallel: bool,
    metric: Metric,
    modifier: Modifier,
    average: Average,
    warn_for: Vec<Metric>,
    zero_division: DivisionByZeroResultStrategy,
) {
    let result = if parallel {
        par_prf_divide_results_and_mask(numerator, denominator)
    } else {
        prf_divide_results(numerator, denominator)
    };
    // mask = denominator == 0.0
    // denominator = denominator.copy()
    // denominator[mask] = 1  # avoid infs/nans
    // result = numerator / denominator

    // if not np.any(mask):
    //     return result

    // # if ``zero_division=1``, set those with denominator == 0 equal to 1
    // result[mask] = 0.0 if zero_division in ['warn', 0] else 1.0
}

/// This function computes the result in parallel. For a synchronous
/// version of this function, see `prf_divide_results``
///
/// * `numerator`: Numerator of the division
/// * `denominator`: denominator of the division
fn par_prf_divide_results_and_mask<F: Float, D: Dimension>(
    numerator: Array<F, D>,
    mut denominator: ArrayViewMut<F, D>,
) -> Array<F, D> {
    denominator.par_mapv_inplace(|v| {
        if v == <F as Float>::zero() {
            <F as Float>::one()
        } else {
            v
        }
    });
    numerator / denominator
}
/// This function computes the result synchronously. For a parallel
/// version of this function, see `par_prf_divide_results``
///
/// * `numerator`: Numerator of the division
/// * `denominator`: denominator of the division
fn prf_divide_nesults_and_mask<F: Float, D: Dimension>(
    numerator: Array<F, D>,
    mut denominator: ArrayViewMut<F, D>,
) -> Array<F, D> {
    denominator.mapv_inplace(|v| {
        if v == <F as Float>::zero() {
            <F as Float>::one()
        } else {
            v
        }
    });
    numerator / denominator
}

#[derive(Debug)]
enum Metric {
    F05,
    F1,
    Precision,
    Recall,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_future_test() {}
}
