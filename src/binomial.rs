//! Regression with a binomial response function. The N parameter must be known ahead of time.
//! This submodule uses const_generics, available only in nightly rust, and must
//! be activated with the "binomial" feature option.
use crate::{
    glm::{Glm, Response},
    link::Transform,
    model::Model,
};
use ndarray::Array1;
use ndarray_linalg::Lapack;
use num_traits::Float;

/// Use a fixed type of u16 for the domain of the binomial distribution.
type BinDom = u16;

/// Binomial regression with a fixed N. Non-canonical link functions are not
/// possible at this time due to the awkward ergonomics with the const trait
/// parameter N.
pub struct Binomial<const N: BinDom>;

impl<const N: BinDom> Response<Binomial<N>> for BinDom {
    fn to_float<F: Float>(self) -> F {
        F::from(self).unwrap()
    }
}

impl<const N: BinDom> Glm for Binomial<N> {
    /// Only the canonical link function is available for binomial regression.
    type Link = link::Logit;

    fn variance<F: Float>(mean: F) -> F {
        let n_float: F = F::from(N).unwrap();
        mean * (n_float - mean) / n_float
    }

    /// The binomial likelihood includes a BetaLn() term of N and y, which can
    /// be skipped for parameter minimization.
    fn log_like_params<F>(data: &Model<Self, F>, regressors: &Array1<F>) -> F
    where
        F: Float + Lapack,
    {
        let lin_pred: Array1<F> = data.linear_predictor(&regressors);
        // When generalizing link functions we'll need to make sure to change this
        let eta: Array1<F> = link::Logit::nat_param(lin_pred);
        // in the canonical version, the natural parameter is logit(p)
        let log_like_sum = (&data.y * &eta).sum()
            - F::from(N).unwrap() * eta.mapv_into(Float::exp).mapv_into(F::ln_1p).sum();
        log_like_sum
    }
}

pub mod link {
    use super::*;
    use crate::link::{Canonical, Link};
    use num_traits::Float;

    pub struct Logit {}
    impl Canonical for Logit {}
    impl<const N: BinDom> Link<Binomial<N>> for Logit {
        fn func<F: Float>(y: Array1<F>) -> Array1<F> {
            let n_float: F = F::from(N).unwrap();
            y.mapv_into(|y| Float::ln(y / (n_float - y)))
        }
        fn func_inv<F: Float>(lin_pred: Array1<F>) -> Array1<F> {
            let n_float: F = F::from(N).unwrap();
            lin_pred.mapv_into(|xb| n_float / (F::one() + (-xb).exp()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Binomial;
    use crate::{error::RegressionResult, model::ModelBuilder};
    use approx::assert_abs_diff_eq;
    use ndarray::array;

    #[test]
    fn bin_reg() -> RegressionResult<()> {
        const N: u16 = 12;
        let ln2 = f64::ln(2.);
        let beta = array![0., 1.];
        let data_x = array![[0.], [0.], [ln2], [ln2], [ln2]];
        // the first two data points should average to 6 and the last 3 should average to 8.
        let data_y = array![5, 7, 9, 6, 9];
        let model = ModelBuilder::<Binomial<N>>::data(&data_y, &data_x).build()?;
        let fit = model.fit()?;
        dbg!(&fit.result);
        dbg!(&fit.n_iter);
        assert_abs_diff_eq!(beta, fit.result, epsilon = 0.05 * std::f32::EPSILON as f64);
        Ok(())
    }
}
