use ndarray::prelude::*;
use ndarray::Array;
use num::Float as NumFloat;
use std::error::Error;
use std::fmt::Display;
use std::sync::OnceLock;

struct PerClassScore(Vec<f32>, Vec<f32>, Vec<f32>, Vec<i32>);
struct AverageScore(f32, f32, f32, i32);

enum Scores {
    PerClassScore(PerClassScore),
    AverageScore(AverageScore),
}

#[derive(Debug)]
enum DivisionByZeroStrategy {
    /// Replace denominator equal to 0 by 1 for the calculations
    ReplaceBy1,
    /// Returns an error
    ReturnError,
}
impl Default for DivisionByZeroStrategy {
    fn default() -> Self {
        Self::ReplaceBy1
    }
}

#[derive(Debug, Clone)]
struct DivisionByZeroError;

impl Display for DivisionByZeroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Encountered division by zero")
    }
}

impl Error for DivisionByZeroError {}

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
    denominator: ArrayViewMut<F, D>,
    parallel: bool,
    metric: Metric,
    modifier: Modifier,
    average: Average,
    warn_for: Vec<Metric>,
    zero_division: DivisionByZeroStrategy,
) -> Result<(), DivisionByZeroError> {
    let (result, found_0_in_denom) = if parallel {
        par_prf_divide_results_and_mask(numerator, denominator)
    } else {
        prf_divide_results_and_mask(numerator, denominator)
    };
    if found_0_in_denom {
        match zero_division {
            DivisionByZeroStrategy::ReturnError => Err(DivisionByZeroError),
            DivisionByZeroStrategy::ReplaceBy1 => {

            }
        }
    }
    // if ``zero_division=1``, set those with denominator == 0 equal to 1
    //result[mask] = 0.0 if zero_division in ['warn', 0] else 1.0

    // the user will be removing warnings if zero_division is set to something
    // different than its default value. If we are computing only f-score
    // the warning will be raised only if precision and recall are ill-defined
    //if zero_division != 'warn' or metric not in warn_for:
    //    return result

    // build appropriate warning
    // E.g. "Precision and F-score are ill-defined and being set to 0.0 in
    // labels with no predicted samples. Use ``zero_division`` parameter to
    // control this behavior."

    //if metric in warn_for and 'f-score' in warn_for:
    //    msg_start = '{0} and F-score are'.format(metric.title())
    //elif metric in warn_for:
    //    msg_start = '{0} is'.format(metric.title())
    //elif 'f-score' in warn_for:
    //    msg_start = 'F-score is'
    //else:
    //    return result

    //_warn_prf(average, modifier, msg_start, len(result))

    //return result
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
