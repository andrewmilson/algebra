use crate::{biginteger::BigInteger, fields::utils::k_adicity, UniformRand};
use ark_serialize::{
    CanonicalDeserialize, CanonicalDeserializeWithFlags, CanonicalSerialize,
    CanonicalSerializeWithFlags, EmptyFlags, Flags,
};
use ark_std::{
    cmp::min,
    fmt::{Debug, Display},
    hash::Hash,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    str::FromStr,
    vec::Vec,
};

pub use ark_ff_macros;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use zeroize::Zeroize;

pub mod utils;

#[macro_use]
pub mod arithmetic;

#[macro_use]
pub mod models;
pub use self::models::*;

pub mod field_hashers;

#[cfg(feature = "parallel")]
use ark_std::cmp::max;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// The interface for a generic field.  
/// Types implementing [`Field`] support common field operations such as addition, subtraction, multiplication, and inverses.
///
/// ## Defining your own field
/// To demonstrate the various field operations, we can first define a prime ordered field $\mathbb{F}_{p}$ with $p = 17$. When defining a field $\mathbb{F}_p$, we need to provide the modulus(the $p$ in $\mathbb{F}_p$) and a generator. Recall that a generator $g \in \mathbb{F}_p$ is a field element whose powers comprise the entire field: $\mathbb{F}_p =\\{g, g^1, \ldots, g^{p-1}\\}$.
/// We can then manually construct the field element associated with an integer with `Fp::from` and perform field addition, subtraction, multiplication, and inversion on it.
/// ```rust
/// use ark_ff::fields::{Field, Fp64, MontBackend, MontConfig};
///
/// #[derive(MontConfig)]
/// #[modulus = "17"]
/// #[generator = "3"]
/// pub struct FqConfig;
/// pub type Fq = Fp64<MontBackend<FqConfig, 1>>;
///
/// let a = Fq::from(9);
/// let b = Fq::from(10);
///
/// assert_eq!(a, Fq::from(26));          // 26 =  9 mod 17
/// assert_eq!(a - b, Fq::from(16));      // -1 = 16 mod 17
/// assert_eq!(a + b, Fq::from(2));       // 19 =  2 mod 17
/// assert_eq!(a * b, Fq::from(5));       // 90 =  5 mod 17
/// assert_eq!(a.square(), Fq::from(13)); // 81 = 13 mod 17
/// assert_eq!(b.double(), Fq::from(3));  // 20 =  3 mod 17
/// assert_eq!(a / b, a * b.inverse().unwrap()); // need to unwrap since `b` could be 0 which is not invertible
/// ```
///
/// ## Using pre-defined fields
/// In the following example, we’ll use the field associated with the BLS12-381 pairing-friendly group.
/// ```rust
/// use ark_ff::Field;
/// use ark_test_curves::bls12_381::Fq as F;
/// use ark_std::{One, UniformRand, test_rng};
///
/// let mut rng = test_rng();
/// // Let's sample uniformly random field elements:
/// let a = F::rand(&mut rng);
/// let b = F::rand(&mut rng);
///
/// let c = a + b;
/// let d = a - b;
/// assert_eq!(c + d, a.double());
///
/// let e = c * d;
/// assert_eq!(e, a.square() - b.square());         // (a + b)(a - b) = a^2 - b^2
/// assert_eq!(a.inverse().unwrap() * a, F::one()); // Euler-Fermat theorem tells us: a * a^{-1} = 1 mod p
/// ```
pub trait Field:
    'static
    + Copy
    + Clone
    + Debug
    + Display
    + Default
    + Send
    + Sync
    + Eq
    + Zero
    + One
    + Ord
    + Neg<Output = Self>
    + UniformRand
    + Zeroize
    + Sized
    + Hash
    + CanonicalSerialize
    + CanonicalSerializeWithFlags
    + CanonicalDeserialize
    + CanonicalDeserializeWithFlags
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Mul<Self, Output = Self>
    + Div<Self, Output = Self>
    + AddAssign<Self>
    + SubAssign<Self>
    + MulAssign<Self>
    + DivAssign<Self>
    + for<'a> Add<&'a Self, Output = Self>
    + for<'a> Sub<&'a Self, Output = Self>
    + for<'a> Mul<&'a Self, Output = Self>
    + for<'a> Div<&'a Self, Output = Self>
    + for<'a> AddAssign<&'a Self>
    + for<'a> SubAssign<&'a Self>
    + for<'a> MulAssign<&'a Self>
    + for<'a> DivAssign<&'a Self>
    + for<'a> Add<&'a mut Self, Output = Self>
    + for<'a> Sub<&'a mut Self, Output = Self>
    + for<'a> Mul<&'a mut Self, Output = Self>
    + for<'a> Div<&'a mut Self, Output = Self>
    + for<'a> AddAssign<&'a mut Self>
    + for<'a> SubAssign<&'a mut Self>
    + for<'a> MulAssign<&'a mut Self>
    + for<'a> DivAssign<&'a mut Self>
    + core::iter::Sum<Self>
    + for<'a> core::iter::Sum<&'a Self>
    + core::iter::Product<Self>
    + for<'a> core::iter::Product<&'a Self>
    + From<u128>
    + From<u64>
    + From<u32>
    + From<u16>
    + From<u8>
    + From<bool>
{
    type BasePrimeField: PrimeField;

    type BasePrimeFieldIter: Iterator<Item = Self::BasePrimeField>;

    /// Determines the algorithm for computing square roots.
    const SQRT_PRECOMP: Option<SqrtPrecomputation<Self>>;

    /// The additive identity of the field.
    const ZERO: Self;
    /// The multiplicative identity of the field.
    const ONE: Self;

    /// Returns the characteristic of the field,
    /// in little-endian representation.
    fn characteristic() -> &'static [u64] {
        Self::BasePrimeField::characteristic()
    }

    /// Returns the extension degree of this field with respect
    /// to `Self::BasePrimeField`.
    fn extension_degree() -> u64;

    fn to_base_prime_field_elements(&self) -> Self::BasePrimeFieldIter;

    /// Convert a slice of base prime field elements into a field element.
    /// If the slice length != Self::extension_degree(), must return None.
    fn from_base_prime_field_elems(elems: &[Self::BasePrimeField]) -> Option<Self>;

    /// Constructs a field element from a single base prime field elements.
    /// ```
    /// # use ark_ff::Field;
    /// # use ark_test_curves::bls12_381::Fq as F;
    /// # use ark_test_curves::bls12_381::Fq2 as F2;
    /// # use ark_std::One;
    /// assert_eq!(F2::from_base_prime_field(F::one()), F2::one());
    /// ```
    fn from_base_prime_field(elem: Self::BasePrimeField) -> Self;

    /// Returns `self + self`.
    #[must_use]
    fn double(&self) -> Self;

    /// Doubles `self` in place.
    fn double_in_place(&mut self) -> &mut Self;

    /// Negates `self` in place.
    fn neg_in_place(&mut self) -> &mut Self;

    /// Attempt to deserialize a field element. Returns `None` if the
    /// deserialization fails.
    ///
    /// This function is primarily intended for sampling random field elements
    /// from a hash-function or RNG output.
    fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
        Self::from_random_bytes_with_flags::<EmptyFlags>(bytes).map(|f| f.0)
    }

    /// Attempt to deserialize a field element, splitting the bitflags metadata
    /// according to `F` specification. Returns `None` if the deserialization
    /// fails.
    ///
    /// This function is primarily intended for sampling random field elements
    /// from a hash-function or RNG output.
    fn from_random_bytes_with_flags<F: Flags>(bytes: &[u8]) -> Option<(Self, F)>;

    /// Returns a `LegendreSymbol`, which indicates whether this field element
    /// is  1 : a quadratic residue
    ///  0 : equal to 0
    /// -1 : a quadratic non-residue
    fn legendre(&self) -> LegendreSymbol;

    /// Returns the square root of self, if it exists.
    #[must_use]
    fn sqrt(&self) -> Option<Self> {
        match Self::SQRT_PRECOMP {
            Some(tv) => tv.sqrt(self),
            None => unimplemented!(),
        }
    }

    /// Sets `self` to be the square root of `self`, if it exists.
    fn sqrt_in_place(&mut self) -> Option<&mut Self> {
        (*self).sqrt().map(|sqrt| {
            *self = sqrt;
            self
        })
    }

    /// Returns `self * self`.
    #[must_use]
    fn square(&self) -> Self;

    /// Squares `self` in place.
    fn square_in_place(&mut self) -> &mut Self;

    /// Computes the multiplicative inverse of `self` if `self` is nonzero.
    #[must_use]
    fn inverse(&self) -> Option<Self>;

    /// If `self.inverse().is_none()`, this just returns `None`. Otherwise, it sets
    /// `self` to `self.inverse().unwrap()`.
    fn inverse_in_place(&mut self) -> Option<&mut Self>;

    /// Returns `sum([a_i * b_i])`.
    #[inline]
    fn sum_of_products<const T: usize>(a: &[Self; T], b: &[Self; T]) -> Self {
        let mut sum = Self::zero();
        for i in 0..a.len() {
            sum += a[i] * b[i];
        }
        sum
    }

    /// Exponentiates this element by a power of the base prime modulus via
    /// the Frobenius automorphism.
    fn frobenius_map(&mut self, power: usize);

    /// Exponentiates this element by a number represented with `u64` limbs,
    /// least significant limb first.
    #[must_use]
    fn pow<S: AsRef<[u64]>>(&self, exp: S) -> Self {
        let mut res = Self::one();

        for i in BitIteratorBE::without_leading_zeros(exp) {
            res.square_in_place();

            if i {
                res *= self;
            }
        }
        res
    }

    /// Exponentiates a field element `f` by a number represented with `u64`
    /// limbs, using a precomputed table containing as many powers of 2 of
    /// `f` as the 1 + the floor of log2 of the exponent `exp`, starting
    /// from the 1st power. That is, `powers_of_2` should equal `&[p, p^2,
    /// p^4, ..., p^(2^n)]` when `exp` has at most `n` bits.
    ///
    /// This returns `None` when a power is missing from the table.
    #[inline]
    fn pow_with_table<S: AsRef<[u64]>>(powers_of_2: &[Self], exp: S) -> Option<Self> {
        let mut res = Self::one();
        for (pow, bit) in BitIteratorLE::without_trailing_zeros(exp).enumerate() {
            if bit {
                res *= powers_of_2.get(pow)?;
            }
        }
        Some(res)
    }
}

/// Fields that have a cyclotomic multiplicative subgroup, and which can
/// leverage efficient inversion and squaring algorithms for elements in this subgroup.
/// If a field has multiplicative order p^d - 1, the cyclotomic subgroups refer to subgroups of order φ_n(p),
/// for any n < d, where φ_n is the [n-th cyclotomic polynomial](https://en.wikipedia.org/wiki/Cyclotomic_polynomial).
///
/// ## Note
///
/// Note that this trait is unrelated to the `Group` trait from the `ark_ec` crate. That trait
/// denotes an *additive* group, while this trait denotes a *multiplicative* group.
pub trait CyclotomicMultSubgroup: Field {
    /// Is the inverse fast to compute? For example, in quadratic extensions, the inverse
    /// can be computed at the cost of negating one coordinate, which is much faster than
    /// standard inversion.
    /// By default this is `false`, but should be set to `true` for quadratic extensions.
    const INVERSE_IS_FAST: bool = false;

    /// Compute a square in the cyclotomic subgroup. By default this is computed using [`Field::square`], but for
    /// degree 12 extensions, this can be computed faster than normal squaring.
    ///
    /// # Warning
    ///
    /// This method should be invoked only when `self` is in the cyclotomic subgroup.
    fn cyclotomic_square(&self) -> Self {
        let mut result = *self;
        *result.cyclotomic_square_in_place()
    }

    /// Square `self` in place. By default this is computed using
    /// [`Field::square_in_place`], but for degree 12 extensions,
    /// this can be computed faster than normal squaring.
    ///
    /// # Warning
    ///
    /// This method should be invoked only when `self` is in the cyclotomic subgroup.
    fn cyclotomic_square_in_place(&mut self) -> &mut Self {
        self.square_in_place()
    }

    /// Compute the inverse of `self`. See [`Self::INVERSE_IS_FAST`] for details.
    /// Returns [`None`] if `self.is_zero()`, and [`Some`] otherwise.
    ///
    /// # Warning
    ///
    /// This method should be invoked only when `self` is in the cyclotomic subgroup.
    fn cyclotomic_inverse(&self) -> Option<Self> {
        let mut result = *self;
        result.cyclotomic_inverse_in_place().copied()
    }

    /// Compute the inverse of `self`. See [`Self::INVERSE_IS_FAST`] for details.
    /// Returns [`None`] if `self.is_zero()`, and [`Some`] otherwise.
    ///
    /// # Warning
    ///
    /// This method should be invoked only when `self` is in the cyclotomic subgroup.
    fn cyclotomic_inverse_in_place(&mut self) -> Option<&mut Self> {
        self.inverse_in_place()
    }

    /// Compute a cyclotomic exponentiation of `self` with respect to `e`.
    ///
    /// # Warning
    ///
    /// This method should be invoked only when `self` is in the cyclotomic subgroup.
    fn cyclotomic_exp(&self, e: impl AsRef<[u64]>) -> Self {
        let mut result = *self;
        result.cyclotomic_exp_in_place(e);
        result
    }

    /// Set `self` to be the result of exponentiating [`self`] by `e`,
    /// using efficient cyclotomic algorithms.
    ///
    /// # Warning
    ///
    /// This method should be invoked only when `self` is in the cyclotomic subgroup.
    fn cyclotomic_exp_in_place(&mut self, e: impl AsRef<[u64]>) {
        if self.is_zero() {
            return;
        }

        if Self::INVERSE_IS_FAST {
            // We only use NAF-based exponentiation if inverses are fast to compute.
            let naf = crate::biginteger::arithmetic::find_naf(e.as_ref());
            exp_loop(self, naf.into_iter().rev())
        } else {
            exp_loop(
                self,
                BitIteratorBE::without_leading_zeros(e.as_ref()).map(|e| e as i8),
            )
        };
    }
}

/// Helper function to calculate the double-and-add loop for exponentiation.
fn exp_loop<F: CyclotomicMultSubgroup, I: Iterator<Item = i8>>(f: &mut F, e: I) {
    // If the inverse is fast and we're using naf, we compute the inverse of the base.
    // Otherwise we do nothing with the variable, so we default it to one.
    let self_inverse = if F::INVERSE_IS_FAST {
        f.cyclotomic_inverse().unwrap() // The inverse must exist because self is not zero.
    } else {
        F::one()
    };
    let mut res = F::one();
    let mut found_nonzero = false;
    for value in e {
        if found_nonzero {
            res.cyclotomic_square_in_place();
        }

        if value != 0 {
            found_nonzero = true;

            if value > 0 {
                res *= &*f;
            } else if F::INVERSE_IS_FAST {
                // only use naf if inversion is fast.
                res *= &self_inverse;
            }
        }
    }
    *f = res;
}

/// The interface for fields that are able to be used in FFTs.
pub trait FftField: Field {
    /// The generator of the multiplicative group of the field
    const GENERATOR: Self;

    /// Let `N` be the size of the multiplicative group defined by the field.
    /// Then `TWO_ADICITY` is the two-adicity of `N`, i.e. the integer `s`
    /// such that `N = 2^s * t` for some odd integer `t`.
    const TWO_ADICITY: u32;

    /// 2^s root of unity computed by GENERATOR^t
    const TWO_ADIC_ROOT_OF_UNITY: Self;

    /// An integer `b` such that there exists a multiplicative subgroup
    /// of size `b^k` for some integer `k`.
    const SMALL_SUBGROUP_BASE: Option<u32> = None;

    /// The integer `k` such that there exists a multiplicative subgroup
    /// of size `Self::SMALL_SUBGROUP_BASE^k`.
    const SMALL_SUBGROUP_BASE_ADICITY: Option<u32> = None;

    /// GENERATOR^((MODULUS-1) / (2^s *
    /// SMALL_SUBGROUP_BASE^SMALL_SUBGROUP_BASE_ADICITY)) Used for mixed-radix
    /// FFT.
    const LARGE_SUBGROUP_ROOT_OF_UNITY: Option<Self> = None;

    /// Returns the root of unity of order n, if one exists.
    /// If no small multiplicative subgroup is defined, this is the 2-adic root
    /// of unity of order n (for n a power of 2).
    /// If a small multiplicative subgroup is defined, this is the root of unity
    /// of order n for the larger subgroup generated by
    /// `FftConfig::LARGE_SUBGROUP_ROOT_OF_UNITY`
    /// (for n = 2^i * FftConfig::SMALL_SUBGROUP_BASE^j for some i, j).
    fn get_root_of_unity(n: u64) -> Option<Self> {
        let mut omega: Self;
        if let Some(large_subgroup_root_of_unity) = Self::LARGE_SUBGROUP_ROOT_OF_UNITY {
            let q = Self::SMALL_SUBGROUP_BASE.expect(
                "LARGE_SUBGROUP_ROOT_OF_UNITY should only be set in conjunction with SMALL_SUBGROUP_BASE",
            ) as u64;
            let small_subgroup_base_adicity = Self::SMALL_SUBGROUP_BASE_ADICITY.expect(
                "LARGE_SUBGROUP_ROOT_OF_UNITY should only be set in conjunction with SMALL_SUBGROUP_BASE_ADICITY",
            );

            let q_adicity = k_adicity(q, n);
            let q_part = q.checked_pow(q_adicity)?;

            let two_adicity = k_adicity(2, n);
            let two_part = 2u64.checked_pow(two_adicity)?;

            if n != two_part * q_part
                || (two_adicity > Self::TWO_ADICITY)
                || (q_adicity > small_subgroup_base_adicity)
            {
                return None;
            }

            omega = large_subgroup_root_of_unity;
            for _ in q_adicity..small_subgroup_base_adicity {
                omega = omega.pow(&[q as u64]);
            }

            for _ in two_adicity..Self::TWO_ADICITY {
                omega.square_in_place();
            }
        } else {
            // Compute the next power of 2.
            let size = n.next_power_of_two() as u64;
            let log_size_of_group = ark_std::log2(usize::try_from(size).expect("too large"));

            if n != size || log_size_of_group > Self::TWO_ADICITY {
                return None;
            }

            // Compute the generator for the multiplicative subgroup.
            // It should be 2^(log_size_of_group) root of unity.
            omega = Self::TWO_ADIC_ROOT_OF_UNITY;
            for _ in log_size_of_group..Self::TWO_ADICITY {
                omega.square_in_place();
            }
        }
        Some(omega)
    }
}

/// The interface for a prime field, i.e. the field of integers modulo a prime $p$.  
/// In the following example we'll use the prime field underlying the BLS12-381 G1 curve.
/// ```rust
/// use ark_ff::{Field, PrimeField, BigInteger};
/// use ark_test_curves::bls12_381::Fq as F;
/// use ark_std::{One, Zero, UniformRand, test_rng};
///
/// let mut rng = test_rng();
/// let a = F::rand(&mut rng);
/// // We can access the prime modulus associated with `F`:
/// let modulus = <F as PrimeField>::MODULUS;
/// assert_eq!(a.pow(&modulus), a); // the Euler-Fermat theorem tells us: a^{p-1} = 1 mod p
///
/// // We can convert field elements to integers in the range [0, MODULUS - 1]:
/// let one: num_bigint::BigUint = F::one().into();
/// assert_eq!(one, num_bigint::BigUint::one());
///
/// // We can construct field elements from an arbitrary sequence of bytes:
/// let n = F::from_le_bytes_mod_order(&modulus.to_bytes_le());
/// assert_eq!(n, F::zero());
/// ```
pub trait PrimeField:
    Field<BasePrimeField = Self>
    + FftField
    + FromStr
    + From<<Self as PrimeField>::BigInt>
    + Into<<Self as PrimeField>::BigInt>
    + From<BigUint>
    + Into<BigUint>
{
    /// A `BigInteger` type that can represent elements of this field.
    type BigInt: BigInteger;

    /// The modulus `p`.
    const MODULUS: Self::BigInt;

    /// The value `(p - 1)/ 2`.
    const MODULUS_MINUS_ONE_DIV_TWO: Self::BigInt;

    /// The size of the modulus in bits.
    const MODULUS_BIT_SIZE: u32;

    /// The trace of the field is defined as the smallest integer `t` such that by
    /// `2^s * t = p - 1`, and `t` is coprime to 2.
    const TRACE: Self::BigInt;
    /// The value `(t - 1)/ 2`.
    const TRACE_MINUS_ONE_DIV_TWO: Self::BigInt;

    /// Construct a prime field element from an integer in the range 0..(p - 1).
    fn from_bigint(repr: Self::BigInt) -> Option<Self>;

    /// Converts an element of the prime field into an integer in the range 0..(p - 1).
    fn into_bigint(self) -> Self::BigInt;

    /// Reads bytes in big-endian, and converts them to a field element.
    /// If the integer represented by `bytes` is larger than the modulus `p`, this method
    /// performs the appropriate reduction.
    fn from_be_bytes_mod_order(bytes: &[u8]) -> Self {
        let num_modulus_bytes = ((Self::MODULUS_BIT_SIZE + 7) / 8) as usize;
        let num_bytes_to_directly_convert = min(num_modulus_bytes - 1, bytes.len());
        // Copy the leading big-endian bytes directly into a field element.
        // The number of bytes directly converted must be less than the
        // number of bytes needed to represent the modulus, as we must begin
        // modular reduction once the data is of the same number of bytes as the
        // modulus.
        let mut bytes_to_directly_convert = Vec::new();
        bytes_to_directly_convert.extend(bytes[..num_bytes_to_directly_convert].iter().rev());
        // Guaranteed to not be None, as the input is less than the modulus size.
        let mut res = Self::from_random_bytes(&bytes_to_directly_convert).unwrap();

        // Update the result, byte by byte.
        // We go through existing field arithmetic, which handles the reduction.
        // TODO: If we need higher speeds, parse more bytes at once, or implement
        // modular multiplication by a u64
        let window_size = Self::from(256u64);
        for byte in bytes[num_bytes_to_directly_convert..].iter() {
            res *= window_size;
            res += Self::from(*byte);
        }
        res
    }

    /// Reads bytes in little-endian, and converts them to a field element.
    /// If the integer represented by `bytes` is larger than the modulus `p`, this method
    /// performs the appropriate reduction.
    fn from_le_bytes_mod_order(bytes: &[u8]) -> Self {
        let mut bytes_copy = bytes.to_vec();
        bytes_copy.reverse();
        Self::from_be_bytes_mod_order(&bytes_copy)
    }
}

/// Indication of the field element's quadratic residuosity
///
/// # Examples
/// ```
/// # use ark_std::test_rng;
/// # use ark_std::UniformRand;
/// # use ark_test_curves::{LegendreSymbol, Field, bls12_381::Fq as Fp};
/// let a: Fp = Fp::rand(&mut test_rng());
/// let b = a.square();
/// assert_eq!(b.legendre(), LegendreSymbol::QuadraticResidue);
/// ```
#[derive(Debug, PartialEq, Eq)]
pub enum LegendreSymbol {
    Zero = 0,
    QuadraticResidue = 1,
    QuadraticNonResidue = -1,
}

impl LegendreSymbol {
    /// Returns true if `self.is_zero()`.
    ///
    /// # Examples
    /// ```
    /// # use ark_std::test_rng;
    /// # use ark_std::UniformRand;
    /// # use ark_test_curves::{LegendreSymbol, Field, bls12_381::Fq as Fp};
    /// let a: Fp = Fp::rand(&mut test_rng());
    /// let b: Fp = a.square();
    /// assert!(!b.legendre().is_zero());
    /// ```
    pub fn is_zero(&self) -> bool {
        *self == LegendreSymbol::Zero
    }

    /// Returns true if `self` is a quadratic non-residue.
    ///
    /// # Examples
    /// ```
    /// # use ark_test_curves::{Fp2Config, Field, LegendreSymbol, bls12_381::{Fq, Fq2Config}};
    /// let a: Fq = Fq2Config::NONRESIDUE;
    /// assert!(a.legendre().is_qnr());
    /// ```
    pub fn is_qnr(&self) -> bool {
        *self == LegendreSymbol::QuadraticNonResidue
    }

    /// Returns true if `self` is a quadratic residue.
    /// # Examples
    /// ```
    /// # use ark_std::test_rng;
    /// # use ark_test_curves::bls12_381::Fq as Fp;
    /// # use ark_std::UniformRand;
    /// # use ark_ff::{LegendreSymbol, Field};
    /// let a: Fp = Fp::rand(&mut test_rng());
    /// let b: Fp = a.square();
    /// assert!(b.legendre().is_qr());
    /// ```
    pub fn is_qr(&self) -> bool {
        *self == LegendreSymbol::QuadraticResidue
    }
}

/// Precomputation that makes computing square roots faster
/// A particular variant should only be instantiated if the modulus satisfies
/// the corresponding condition.
#[non_exhaustive]
pub enum SqrtPrecomputation<F: Field> {
    // Tonelli-Shanks algorithm works for all elements, no matter what the modulus is.
    TonelliShanks {
        two_adicity: u32,
        quadratic_nonresidue_to_trace: F,
        trace_of_modulus_minus_one_div_two: &'static [u64],
    },
    /// To be used when the modulus is 3 mod 4.
    Case3Mod4 {
        modulus_plus_one_div_four: &'static [u64],
    },
}

impl<F: Field> SqrtPrecomputation<F> {
    fn sqrt(&self, elem: &F) -> Option<F> {
        match self {
            Self::TonelliShanks {
                two_adicity,
                quadratic_nonresidue_to_trace,
                trace_of_modulus_minus_one_div_two,
            } => {
                // https://eprint.iacr.org/2012/685.pdf (page 12, algorithm 5)
                // Actually this is just normal Tonelli-Shanks; since `P::Generator`
                // is a quadratic non-residue, `P::ROOT_OF_UNITY = P::GENERATOR ^ t`
                // is also a quadratic non-residue (since `t` is odd).
                if elem.is_zero() {
                    return Some(F::zero());
                }
                // Try computing the square root (x at the end of the algorithm)
                // Check at the end of the algorithm if x was a square root
                // Begin Tonelli-Shanks
                let mut z = *quadratic_nonresidue_to_trace;
                let mut w = elem.pow(trace_of_modulus_minus_one_div_two);
                let mut x = w * elem;
                let mut b = x * &w;

                let mut v = *two_adicity as usize;

                while !b.is_one() {
                    let mut k = 0usize;

                    let mut b2k = b;
                    while !b2k.is_one() {
                        // invariant: b2k = b^(2^k) after entering this loop
                        b2k.square_in_place();
                        k += 1;
                    }

                    if k == (*two_adicity as usize) {
                        // We are in the case where self^(T * 2^k) = x^(P::MODULUS - 1) = 1,
                        // which means that no square root exists.
                        return None;
                    }
                    let j = v - k;
                    w = z;
                    for _ in 1..j {
                        w.square_in_place();
                    }

                    z = w.square();
                    b *= &z;
                    x *= &w;
                    v = k;
                }
                // Is x the square root? If so, return it.
                if x.square() == *elem {
                    Some(x)
                } else {
                    // Consistency check that if no square root is found,
                    // it is because none exists.
                    debug_assert!(!matches!(elem.legendre(), LegendreSymbol::QuadraticResidue));
                    None
                }
            },
            Self::Case3Mod4 {
                modulus_plus_one_div_four,
            } => {
                let result = elem.pow(modulus_plus_one_div_four.as_ref());
                (result.square() == *elem).then_some(result)
            },
        }
    }
}

/// Iterates over a slice of `u64` in *big-endian* order.
#[derive(Debug)]
pub struct BitIteratorBE<Slice: AsRef<[u64]>> {
    s: Slice,
    n: usize,
}

impl<Slice: AsRef<[u64]>> BitIteratorBE<Slice> {
    pub fn new(s: Slice) -> Self {
        let n = s.as_ref().len() * 64;
        BitIteratorBE { s, n }
    }

    /// Construct an iterator that automatically skips any leading zeros.
    /// That is, it skips all zeros before the most-significant one.
    pub fn without_leading_zeros(s: Slice) -> impl Iterator<Item = bool> {
        Self::new(s).skip_while(|b| !b)
    }
}

impl<Slice: AsRef<[u64]>> Iterator for BitIteratorBE<Slice> {
    type Item = bool;

    fn next(&mut self) -> Option<bool> {
        if self.n == 0 {
            None
        } else {
            self.n -= 1;
            let part = self.n / 64;
            let bit = self.n - (64 * part);

            Some(self.s.as_ref()[part] & (1 << bit) > 0)
        }
    }
}

/// Iterates over a slice of `u64` in *little-endian* order.
#[derive(Debug)]
pub struct BitIteratorLE<Slice: AsRef<[u64]>> {
    s: Slice,
    n: usize,
    max_len: usize,
}

impl<Slice: AsRef<[u64]>> BitIteratorLE<Slice> {
    pub fn new(s: Slice) -> Self {
        let n = 0;
        let max_len = s.as_ref().len() * 64;
        BitIteratorLE { s, n, max_len }
    }

    /// Construct an iterator that automatically skips any trailing zeros.
    /// That is, it skips all zeros after the most-significant one.
    pub fn without_trailing_zeros(s: Slice) -> impl Iterator<Item = bool> {
        let mut first_trailing_zero = 0;
        for (i, limb) in s.as_ref().iter().enumerate().rev() {
            first_trailing_zero = i * 64 + (64 - limb.leading_zeros()) as usize;
            if *limb != 0 {
                break;
            }
        }
        let mut iter = Self::new(s);
        iter.max_len = first_trailing_zero;
        iter
    }
}

impl<Slice: AsRef<[u64]>> Iterator for BitIteratorLE<Slice> {
    type Item = bool;

    fn next(&mut self) -> Option<bool> {
        if self.n == self.max_len {
            None
        } else {
            let part = self.n / 64;
            let bit = self.n - (64 * part);
            self.n += 1;

            Some(self.s.as_ref()[part] & (1 << bit) > 0)
        }
    }
}

// Given a vector of field elements {v_i}, compute the vector {v_i^(-1)}
pub fn batch_inversion<F: Field>(v: &mut [F]) {
    batch_inversion_and_mul(v, &F::one());
}

#[cfg(not(feature = "parallel"))]
// Given a vector of field elements {v_i}, compute the vector {coeff * v_i^(-1)}
pub fn batch_inversion_and_mul<F: Field>(v: &mut [F], coeff: &F) {
    serial_batch_inversion_and_mul(v, coeff);
}

#[cfg(feature = "parallel")]
// Given a vector of field elements {v_i}, compute the vector {coeff * v_i^(-1)}
pub fn batch_inversion_and_mul<F: Field>(v: &mut [F], coeff: &F) {
    // Divide the vector v evenly between all available cores
    let min_elements_per_thread = 1;
    let num_cpus_available = rayon::current_num_threads();
    let num_elems = v.len();
    let num_elem_per_thread = max(num_elems / num_cpus_available, min_elements_per_thread);

    // Batch invert in parallel, without copying the vector
    v.par_chunks_mut(num_elem_per_thread).for_each(|mut chunk| {
        serial_batch_inversion_and_mul(&mut chunk, coeff);
    });
}

/// Given a vector of field elements {v_i}, compute the vector {coeff * v_i^(-1)}.
/// This method is explicitly single-threaded.
fn serial_batch_inversion_and_mul<F: Field>(v: &mut [F], coeff: &F) {
    // Montgomery’s Trick and Fast Implementation of Masked AES
    // Genelle, Prouff and Quisquater
    // Section 3.2
    // but with an optimization to multiply every element in the returned vector by
    // coeff

    // First pass: compute [a, ab, abc, ...]
    let mut prod = Vec::with_capacity(v.len());
    let mut tmp = F::one();
    for f in v.iter().filter(|f| !f.is_zero()) {
        tmp.mul_assign(f);
        prod.push(tmp);
    }

    // Invert `tmp`.
    tmp = tmp.inverse().unwrap(); // Guaranteed to be nonzero.

    // Multiply product by coeff, so all inverses will be scaled by coeff
    tmp *= coeff;

    // Second pass: iterate backwards to compute inverses
    for (f, s) in v.iter_mut()
        // Backwards
        .rev()
        // Ignore normalized elements
        .filter(|f| !f.is_zero())
        // Backwards, skip last element, fill in one for last term.
        .zip(prod.into_iter().rev().skip(1).chain(Some(F::one())))
    {
        // tmp := tmp * f; f := tmp * s = 1/f
        let new_tmp = tmp * *f;
        *f = tmp * &s;
        tmp = new_tmp;
    }
}

#[cfg(all(test, feature = "std"))]
mod std_tests {
    use super::BitIteratorLE;

    #[test]
    fn bit_iterator_le() {
        let bits = BitIteratorLE::new(&[0, 1 << 10]).collect::<Vec<_>>();
        dbg!(&bits);
        assert!(bits[74]);
        for (i, bit) in bits.into_iter().enumerate() {
            if i != 74 {
                assert!(!bit)
            } else {
                assert!(bit)
            }
        }
    }
}

#[cfg(test)]
mod no_std_tests {
    use super::*;
    use ark_std::test_rng;
    // TODO: only Fr & FrConfig should need to be imported.
    // The rest of imports are caused by cargo not resolving the deps properly
    // from this crate and from ark_test_curves
    use ark_test_curves::{batch_inversion, batch_inversion_and_mul, bls12_381::Fr, PrimeField};

    #[test]
    fn test_batch_inversion() {
        let mut random_coeffs = Vec::<Fr>::new();
        let vec_size = 1000;

        for _ in 0..=vec_size {
            random_coeffs.push(Fr::rand(&mut test_rng()));
        }

        let mut random_coeffs_inv = random_coeffs.clone();
        batch_inversion::<Fr>(&mut random_coeffs_inv);
        for i in 0..=vec_size {
            assert_eq!(random_coeffs_inv[i] * random_coeffs[i], Fr::one());
        }
        let rand_multiplier = Fr::rand(&mut test_rng());
        let mut random_coeffs_inv_shifted = random_coeffs.clone();
        batch_inversion_and_mul(&mut random_coeffs_inv_shifted, &rand_multiplier);
        for i in 0..=vec_size {
            assert_eq!(
                random_coeffs_inv_shifted[i] * random_coeffs[i],
                rand_multiplier
            );
        }
    }

    #[test]
    fn test_from_into_biguint() {
        let mut rng = ark_std::test_rng();

        let modulus_bits = Fr::MODULUS_BIT_SIZE;
        let modulus: num_bigint::BigUint = Fr::MODULUS.into();

        let mut rand_bytes = Vec::new();
        for _ in 0..(2 * modulus_bits / 8) {
            rand_bytes.push(u8::rand(&mut rng));
        }

        let rand = BigUint::from_bytes_le(&rand_bytes);

        let a: BigUint = Fr::from(rand.clone()).into();
        let b = rand % modulus;

        assert_eq!(a, b);
    }

    #[test]
    fn test_from_be_bytes_mod_order() {
        // Each test vector is a byte array,
        // and its tested by parsing it with from_bytes_mod_order, and the num-bigint
        // library. The bytes are currently generated from scripts/test_vectors.py.
        // TODO: Eventually generate all the test vector bytes via computation with the
        // modulus
        use ark_std::{rand::Rng, string::ToString};
        use ark_test_curves::BigInteger;
        use num_bigint::BigUint;

        let ref_modulus = BigUint::from_bytes_be(&Fr::MODULUS.to_bytes_be());

        let mut test_vectors = vec![
            // 0
            vec![0u8],
            // 1
            vec![1u8],
            // 255
            vec![255u8],
            // 256
            vec![1u8, 0u8],
            // 65791
            vec![1u8, 0u8, 255u8],
            // 204827637402836681560342736360101429053478720705186085244545541796635082752
            vec![
                115u8, 237u8, 167u8, 83u8, 41u8, 157u8, 125u8, 72u8, 51u8, 57u8, 216u8, 8u8, 9u8,
                161u8, 216u8, 5u8, 83u8, 189u8, 164u8, 2u8, 255u8, 254u8, 91u8, 254u8, 255u8,
                255u8, 255u8, 255u8, 0u8, 0u8, 0u8,
            ],
            // 204827637402836681560342736360101429053478720705186085244545541796635082753
            vec![
                115u8, 237u8, 167u8, 83u8, 41u8, 157u8, 125u8, 72u8, 51u8, 57u8, 216u8, 8u8, 9u8,
                161u8, 216u8, 5u8, 83u8, 189u8, 164u8, 2u8, 255u8, 254u8, 91u8, 254u8, 255u8,
                255u8, 255u8, 255u8, 0u8, 0u8, 1u8,
            ],
            // 52435875175126190479447740508185965837690552500527637822603658699938581184512
            vec![
                115u8, 237u8, 167u8, 83u8, 41u8, 157u8, 125u8, 72u8, 51u8, 57u8, 216u8, 8u8, 9u8,
                161u8, 216u8, 5u8, 83u8, 189u8, 164u8, 2u8, 255u8, 254u8, 91u8, 254u8, 255u8,
                255u8, 255u8, 255u8, 0u8, 0u8, 0u8, 0u8,
            ],
            // 52435875175126190479447740508185965837690552500527637822603658699938581184513
            vec![
                115u8, 237u8, 167u8, 83u8, 41u8, 157u8, 125u8, 72u8, 51u8, 57u8, 216u8, 8u8, 9u8,
                161u8, 216u8, 5u8, 83u8, 189u8, 164u8, 2u8, 255u8, 254u8, 91u8, 254u8, 255u8,
                255u8, 255u8, 255u8, 0u8, 0u8, 0u8, 1u8,
            ],
            // 52435875175126190479447740508185965837690552500527637822603658699938581184514
            vec![
                115u8, 237u8, 167u8, 83u8, 41u8, 157u8, 125u8, 72u8, 51u8, 57u8, 216u8, 8u8, 9u8,
                161u8, 216u8, 5u8, 83u8, 189u8, 164u8, 2u8, 255u8, 254u8, 91u8, 254u8, 255u8,
                255u8, 255u8, 255u8, 0u8, 0u8, 0u8, 2u8,
            ],
            // 104871750350252380958895481016371931675381105001055275645207317399877162369026
            vec![
                231u8, 219u8, 78u8, 166u8, 83u8, 58u8, 250u8, 144u8, 102u8, 115u8, 176u8, 16u8,
                19u8, 67u8, 176u8, 10u8, 167u8, 123u8, 72u8, 5u8, 255u8, 252u8, 183u8, 253u8,
                255u8, 255u8, 255u8, 254u8, 0u8, 0u8, 0u8, 2u8,
            ],
            // 13423584044832304762738621570095607254448781440135075282586536627184276783235328
            vec![
                115u8, 237u8, 167u8, 83u8, 41u8, 157u8, 125u8, 72u8, 51u8, 57u8, 216u8, 8u8, 9u8,
                161u8, 216u8, 5u8, 83u8, 189u8, 164u8, 2u8, 255u8, 254u8, 91u8, 254u8, 255u8,
                255u8, 255u8, 255u8, 0u8, 0u8, 0u8, 1u8, 0u8,
            ],
            // 115792089237316195423570985008687907853269984665640564039457584007913129639953
            vec![
                1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                17u8,
            ],
            // 168227964412442385903018725516873873690960537166168201862061242707851710824468
            vec![
                1u8, 115u8, 237u8, 167u8, 83u8, 41u8, 157u8, 125u8, 72u8, 51u8, 57u8, 216u8, 8u8,
                9u8, 161u8, 216u8, 5u8, 83u8, 189u8, 164u8, 2u8, 255u8, 254u8, 91u8, 254u8, 255u8,
                255u8, 255u8, 255u8, 0u8, 0u8, 0u8, 20u8,
            ],
            // 29695210719928072218913619902732290376274806626904512031923745164725699769008210
            vec![
                1u8, 0u8, 115u8, 237u8, 167u8, 83u8, 41u8, 157u8, 125u8, 72u8, 51u8, 57u8, 216u8,
                8u8, 9u8, 161u8, 216u8, 5u8, 83u8, 189u8, 164u8, 2u8, 255u8, 254u8, 91u8, 254u8,
                255u8, 255u8, 255u8, 255u8, 0u8, 0u8, 0u8, 82u8,
            ],
        ];
        // Add random bytestrings to the test vector list
        for i in 1..512 {
            let mut rng = test_rng();
            let data: Vec<u8> = (0..i).map(|_| rng.gen()).collect();
            test_vectors.push(data);
        }
        for i in test_vectors {
            let mut expected_biguint = BigUint::from_bytes_be(&i);
            // Reduce expected_biguint using modpow API
            expected_biguint =
                expected_biguint.modpow(&BigUint::from_bytes_be(&[1u8]), &ref_modulus);
            let expected_string = expected_biguint.to_string();
            let expected = Fr::from_str(&expected_string).unwrap();
            let actual = Fr::from_be_bytes_mod_order(&i);
            assert_eq!(expected, actual, "failed on test {:?}", i);
        }
    }
}
