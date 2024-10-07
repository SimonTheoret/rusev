type PerClassScore = (Vec<f32>, Vec<f32>, Vec<f32>, Vec<i32>);
type AverageScore = (f32, f32, f32, i32);

enum Scores {
    PerClassScore,
    AverageScore,
}
