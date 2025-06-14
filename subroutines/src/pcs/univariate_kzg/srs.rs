// Copyright (c) 2023 Espresso Systems (espressosys.com)
// This file is part of the HyperPlonk library.

// You should have received a copy of the MIT License
// along with the HyperPlonk library. If not, see <https://mit-license.org/>.

//! Implementing Structured Reference Strings for univariate polynomial KZG

use crate::pcs::{PCSError, StructuredReferenceString};
use ark_ec::{pairing::Pairing, scalar_mul::BatchMulPreprocessing, AffineRepr, CurveGroup};
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{end_timer, rand::Rng, start_timer, vec, vec::Vec, One, UniformRand};
use derivative::Derivative;
use std::ops::Mul;

/// `UniversalParams` are the universal parameters for the KZG10 scheme.
// Adapted from
// https://github.com/arkworks-rs/poly-commit/blob/master/src/kzg10/data_structures.rs#L20
#[derive(Debug, Clone, Eq, PartialEq, CanonicalSerialize, CanonicalDeserialize, Default)]
pub struct UnivariateUniversalParams<E: Pairing> {
    /// Group elements of the form `{ \beta^i G }`, where `i` ranges from 0 to
    /// `degree`.
    pub powers_of_g: Vec<E::G1Affine>,
    /// The generator of G2.
    pub h: E::G2Affine,
    /// \beta times the above generator of G2.
    pub beta_h: E::G2Affine,
}

impl<E: Pairing> UnivariateUniversalParams<E> {
    /// Returns the maximum supported degree
    pub fn max_degree(&self) -> usize {
        self.powers_of_g.len()
    }
}

/// `UnivariateProverParam` is used to generate a proof
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct UnivariateProverParam<C: AffineRepr> {
    /// Parameters
    pub powers_of_g: Vec<C>,
}

/// `UnivariateVerifierParam` is used to check evaluation proofs for a given
/// commitment.
#[derive(Derivative, CanonicalSerialize, CanonicalDeserialize)]
#[derivative(
    Default(bound = ""),
    Clone(bound = ""),
    Copy(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct UnivariateVerifierParam<E: Pairing> {
    /// The generator of G1.
    pub g: E::G1Affine,
    /// The generator of G2.
    pub h: E::G2Affine,
    /// \beta times the above generator of G2.
    pub beta_h: E::G2Affine,
}

impl<E: Pairing> StructuredReferenceString<E> for UnivariateUniversalParams<E> {
    type ProverParam = UnivariateProverParam<E::G1Affine>;
    type VerifierParam = UnivariateVerifierParam<E>;

    /// Extract the prover parameters from the public parameters.
    fn extract_prover_param(&self, supported_size: usize) -> Self::ProverParam {
        let powers_of_g = self.powers_of_g[..=supported_size].to_vec();

        Self::ProverParam { powers_of_g }
    }

    /// Extract the verifier parameters from the public parameters.
    fn extract_verifier_param(&self, _supported_size: usize) -> Self::VerifierParam {
        Self::VerifierParam {
            g: self.powers_of_g[0],
            h: self.h,
            beta_h: self.beta_h,
        }
    }

    /// Trim the universal parameters to specialize the public parameters
    /// for univariate polynomials to the given `supported_size`, and
    /// returns committer key and verifier key. `supported_size` should
    /// be in range `1..params.len()`
    fn trim(
        &self,
        supported_size: usize,
    ) -> Result<(Self::ProverParam, Self::VerifierParam), PCSError> {
        let powers_of_g = self.powers_of_g[..=supported_size].to_vec();

        let pk = Self::ProverParam { powers_of_g };
        let vk = Self::VerifierParam {
            g: self.powers_of_g[0],
            h: self.h,
            beta_h: self.beta_h,
        };
        Ok((pk, vk))
    }

    /// Build SRS for testing.
    /// WARNING: THIS FUNCTION IS FOR TESTING PURPOSE ONLY.
    /// THE OUTPUT SRS SHOULD NOT BE USED IN PRODUCTION.
    fn gen_srs_for_testing<R: Rng>(rng: &mut R, max_degree: usize) -> Result<Self, PCSError> {
        let setup_time = start_timer!(|| format!("KZG10::Setup with degree {}", max_degree));
        let beta = E::ScalarField::rand(rng);
        let g = E::G1::rand(rng);
        let h = E::G2::rand(rng);

        let mut powers_of_beta = vec![E::ScalarField::one()];

        let mut cur = beta;
        for _ in 0..max_degree {
            powers_of_beta.push(cur);
            cur *= &beta;
        }

        let g_batch_mul_preprocessing = BatchMulPreprocessing::<E::G1>::new(g, max_degree+1);
        let powers_of_g = g_batch_mul_preprocessing.batch_mul(&powers_of_beta);

        let h = h.into_affine();
        let beta_h = h.mul(beta).into_affine();

        let pp = Self {
            powers_of_g,
            h,
            beta_h,
        };
        end_timer!(setup_time);
        Ok(pp)
    }
}
