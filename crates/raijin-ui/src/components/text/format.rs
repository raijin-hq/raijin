// html and html5minify require html5ever/markup5ever_rcdom dependencies
// which are not yet added to Cargo.toml. Gated behind "html" feature.
#[cfg(feature = "html")]
pub(super) mod html;
#[cfg(feature = "html")]
mod html5minify;
pub(super) mod markdown;
