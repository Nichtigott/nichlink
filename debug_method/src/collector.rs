//! Debug-only linker-section backend for NichLink declarations.

#[doc(hidden)]
pub use inventory;

#[macro_export]
macro_rules! collect {
    ($ty:ty) => {
        $crate::inventory::collect!($ty);
    };
}

#[macro_export]
macro_rules! registrations {
    ($ty:ty) => {{
        #[cfg(debug_assertions)]
        {
            $crate::inventory::iter::<$crate::CollectedRegistration>
                .into_iter()
                .map(|entry| entry.0)
        }
        #[cfg(not(debug_assertions))]
        {
            std::iter::empty::<&'static $ty>()
        }
    }};
}

#[macro_export]
macro_rules! submit {
    ($value:expr) => {
        $crate::inventory::submit! {
            $crate::CollectedRegistration(&$value)
        }
    };
}
