// Наряд №110: svg.rs (9329 строк) разбит на модули внутри builtins —
// подготовка к Cargo features (наряд №111). Публичный API не изменился:
// все builtin_* функции продолжают быть доступны как `crate::builtins::svg::*`
// (см. `pub use` ниже), src/builtins/mod.rs и registry.rs не требуют правок.

mod shared;
mod primitives;
mod charts;
mod diagrams;

pub use primitives::*;
pub use charts::*;
pub use diagrams::*;

// extract_style/style_token/canvas_preset — pub(crate) в оригинале,
// нужны другим модулям builtins напрямую.
pub(crate) use shared::{canvas_preset, extract_style, style_token};

#[cfg(test)]
mod tests;
