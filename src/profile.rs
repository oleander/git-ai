#[macro_export] macro_rules! profile { ($name:expr) => { #[cfg(debug_assertions)] let _span = tracing::debug_span!($name).entered(); }; }
