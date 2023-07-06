use std::fmt::{Arguments, Debug};

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

#[track_caller]
#[doc(hidden)]
pub fn approx_eq_assertion<T>(lhs: T, rhs: T, args: Option<Arguments<'_>>)
where
    T: Float + Debug,
{
    if (lhs - rhs).abs() > T::epsilon() {
        approx_eq_assertion_failed(lhs, rhs, args);
    }
}

#[track_caller]
#[doc(hidden)]
fn approx_eq_assertion_failed<T>(lhs: T, rhs: T, args: Option<Arguments<'_>>)
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
