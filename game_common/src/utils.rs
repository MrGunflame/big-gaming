pub mod exclusive;
pub mod vec_ext;

use std::fmt::{Arguments, Debug};

use glam::{Quat, Vec2, Vec3, Vec4};
use num_traits::Float;

/// Floating-point approximate equality comparssion.
///
/// This will expand to the following check:
/// ```
/// # const EPSILON: f32 = f32::EPSILON;
/// #
/// # fn approx_eq(lhs: f32, rhs: f32) -> bool {
/// (lhs - rhs).abs() <= EPSILON
/// # }
/// ```
#[macro_export]
macro_rules! assert_approx_eq {
    // Note that we're going through a function so rustc can infer what
    // type `$lhs` and `$rhs` is. This is necessary to get the epsilon value
    // of the type.
    ($lhs:expr, $rhs:expr $(,)?) => {{
        $crate::utils::approx_eq_assertion($lhs, $rhs, ::core::option::Option::None);
    }};
    ($lhs:expr, $rhs:expr, $($arg:tt)+) => {{
        $crate::utils::approx_eq_assertion($lhs, $rhs, ::core::option::Option::Some(
            ::core::format_args!($($arg)+)
        ));
    }};
}

pub trait ApproxFloatEq {
    type Primitive: Float;
    type ElementIter: Iterator<Item = Self::Primitive>;

    fn elements(&self) -> Self::ElementIter;
}

impl ApproxFloatEq for f32 {
    type Primitive = f32;
    type ElementIter = F32Iter;

    fn elements(&self) -> Self::ElementIter {
        F32Iter { value: Some(*self) }
    }
}

impl ApproxFloatEq for f64 {
    type Primitive = f64;
    type ElementIter = F64Iter;

    fn elements(&self) -> Self::ElementIter {
        F64Iter { value: Some(*self) }
    }
}

pub struct F32Iter {
    value: Option<f32>,
}

impl Iterator for F32Iter {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.value.take()
    }
}

pub struct F64Iter {
    value: Option<f64>,
}

impl Iterator for F64Iter {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        self.value.take()
    }
}

macro_rules! impl_vec {
    ($vec:ty,$iter:ident, $dim:expr) => {
        impl ApproxFloatEq for $vec {
            type Primitive = f32;
            type ElementIter = $iter;

            fn elements(&self) -> $iter {
                $iter {
                    values: self.to_array(),
                    index: 0,
                }
            }
        }

        pub struct $iter {
            values: [f32; $dim],
            index: usize,
        }

        impl Iterator for $iter {
            type Item = f32;

            fn next(&mut self) -> Option<Self::Item> {
                let val = *self.values.get(self.index)?;
                self.index += 1;
                Some(val)
            }
        }
    };
}

impl_vec!(Vec2, Vec2Iter, 2);
impl_vec!(Vec3, Vec3Iter, 3);
impl_vec!(Vec4, Vec4Iter, 4);
impl_vec!(Quat, QuatIter, 4);

#[track_caller]
#[doc(hidden)]
pub fn approx_eq_assertion<T>(lhs: T, rhs: T, args: Option<Arguments<'_>>)
where
    T: ApproxFloatEq + Debug,
{
    for (lhs_elem, rhs_elem) in lhs.elements().zip(rhs.elements()) {
        if (lhs_elem - rhs_elem).abs() > <T::Primitive as Float>::epsilon() {
            approx_eq_assertion_failed(lhs, rhs, args);
        }
    }
}

#[track_caller]
#[doc(hidden)]
fn approx_eq_assertion_failed<T>(lhs: T, rhs: T, args: Option<Arguments<'_>>) -> !
where
    T: Debug,
{
    match args {
        Some(args) => {
            panic!(
                r#"assertion failed: `(left == right)`
                left: `{:?}`
                right: `{:?}`: {}
                "#,
                lhs, rhs, args,
            );
        }
        None => {
            panic!(
                r#"assertion failed: `(left == right)`
                left: `{:?}`
                right: `{:?}`
                "#,
                lhs, rhs
            );
        }
    }
}

mod tests {
    #[test]
    fn assert_approx_eq() {
        let lhs = (-1.0) + ((1.0 - -1.0) * 400.0 / 1000.0);
        assert_approx_eq!(lhs, -0.2);
    }

    #[test]
    #[should_panic]
    fn assert_approx_eq_failure() {
        let lhs = (-1.0) + ((1.0 - -1.0) * 400.0 / 1000.0);
        assert_approx_eq!(lhs, -0.2 - f64::EPSILON);
    }

    #[test]
    fn assert_approx_eq_args() {
        assert_approx_eq!(1.0, 1.0, "oh no!");
    }
}
