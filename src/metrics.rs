use ndarray::parallel::prelude::*;
use ndarray::prelude::*;
use ndarray::Array;
use num::Float as NumFloat;
use std::cell::UnsafeCell;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::LazyLock;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

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
        prf_divide_results_and_mask(numerator, denominator)
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

type Found0InDenominator = bool;

/// This function computes the result in parallel. For a synchronous
/// version of this function, see `prf_divide_results`. The second
/// return argument is `true` if it foufnd a zero in the
/// denominator. Else, it is `false`.
///
/// * `numerator`: Numerator of the division
/// * `denominator`: denominator of the division
fn par_prf_divide_results_and_mask<F: Float, D: Dimension>(
    numerator: Array<F, D>,
    mut denominator: ArrayViewMut<F, D>,
) -> (Array<F, D>, Found0InDenominator) {
    let found_zero_in_denom_cell = OnceLock::new();
    denominator.par_mapv_inplace(|v| {
        if v == <F as Float>::zero() {
            found_zero_in_denom_cell.get_or_init(|| false);
            <F as Float>::one()
        } else {
            v
        }
    });
    let found_zero_in_denom = found_zero_in_denom_cell.into_inner().unwrap_or(false);
    (numerator / denominator, found_zero_in_denom)
}

/// This function computes the result synchronously. For a parallel
/// version of this function, see `par_prf_divide_results`. The second
/// return argument is `true` if it foufnd a zero in the
/// denominator. Else, it is `false`.
///
/// * `numerator`: Numerator of the division
/// * `denominator`: denominator of the division
fn prf_divide_results_and_mask<F: Float, D: Dimension>(
    numerator: Array<F, D>,
    mut denominator: ArrayViewMut<F, D>,
) -> (Array<F, D>, Found0InDenominator) {
    let mut found_zero_in_num: Found0InDenominator = false;
    denominator.mapv_inplace(|v| {
        if v == <F as Float>::zero() {
            found_zero_in_num = true;
            <F as Float>::one()
        } else {
            v
        }
    });
    (numerator / denominator, found_zero_in_num)
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
    fn test_par_divide_results_and_mask() {
        let numerator = array![[1., 2., 4., 5.]];
        let mut cloned = numerator.clone();
        let mut same_cloned = numerator.clone();
        let denominator = cloned.view_mut();
        let same_denominator = same_cloned.view_mut();
        let (div_result, has_zero) =
            prf_divide_results_and_mask(numerator.clone(), same_denominator);
        let (par_div_result, par_has_zero) =
            par_prf_divide_results_and_mask(numerator, denominator);
        let has_no_zero = !has_zero;
        let par_has_no_zero = !par_has_zero;
        assert!(has_no_zero);
        assert!(par_has_no_zero);
        assert_eq!(div_result, array![[1., 1., 1., 1.,]]);
        assert_eq!(par_div_result, array![[1., 1., 1., 1.,]]);
    }
}

