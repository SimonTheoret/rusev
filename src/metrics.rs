use ndarray::prelude::*;
use ndarray::parallel::prelude::*;
use ndarray::Array;

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

/// This enum wraps the floating point type used in the
/// calculation. It excludes f128 because f128 are not well supported
/// in most architectures and is nightly:
/// https://doc.rust-lang.org/nightly/std/primitive.f128.html
#[derive(Debug, Clone, PartialEq, Copy)]
enum FloatingPoint {
    F32,
    F64,
}


impl Default for FloatingPoint {
    fn default() -> Self {
        Self::F32
    }
}

// TODO: Change the warn_for `vec` for a slice (if possible)
fn prf_divide<FloatingPoint, D>(
    numerator: Array<FloatingPoint, D>,
    denominator: Array<FloatingPoint, D>,
    metric: Metric,
    modifier: Modifier,
    average: Average,
    warn_for: Vec<Metric>,
    zero_division: DivisionByZeroResultStrategy,
) {
    todo!()
    // mask = denominator == 0.0
    // denominator = denominator.copy()
    // denominator[mask] = 1  # avoid infs/nans
    // result = numerator / denominator

    // if not np.any(mask):
    //     return result

    // # if ``zero_division=1``, set those with denominator == 0 equal to 1
    // result[mask] = 0.0 if zero_division in ['warn', 0] else 1.0
}

/// This function computes the result in parallel. For a synchronous cersion of this function, see `prf_divide_results``
fn par_prf_divide_results<D: Dimension>(
    numerator: Array<FloatingPoint, D>,
    denominator: Array<FloatingPoint, D>,
) {
    let mask = denominator.par_mapv_inplace(|v| {if v == 0.0 {
        v
    }else {v}} );
}
/// This functon computes the result synchronously. For a parallel function, see `par_prf_divide_results`.
fn prf_divide_results<FloatingPoint, D>(
    numerator: Array<FloatingPoint, D>,
    denominator: Array<FloatingPoint, D>,
) {
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
