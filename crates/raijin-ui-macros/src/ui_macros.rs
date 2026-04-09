mod derive_into_plot;
mod derive_register_component;
mod dynamic_spacing;
mod icon_named;

use proc_macro::TokenStream;

/// Generates the DynamicSpacing enum used for density-aware spacing in the UI.
#[proc_macro]
pub fn derive_dynamic_spacing(input: TokenStream) -> TokenStream {
    dynamic_spacing::derive_spacing(input)
}

/// Registers components that implement the `Component` trait.
///
/// This proc macro is used to automatically register structs that implement
/// the `Component` trait with the [`inazuma_component_registry::ComponentRegistry`].
///
/// If the component trait is not implemented, it will generate a compile-time error.
///
/// # Example
///
/// ```
/// use raijin_ui::Component;
/// use raijin_ui_macros::RegisterComponent;
///
/// #[derive(RegisterComponent)]
/// struct MyComponent;
///
/// impl Component for MyComponent {
///     // Component implementation
/// }
/// ```
///
/// This example will add MyComponent to the ComponentRegistry.
#[proc_macro_derive(RegisterComponent)]
pub fn derive_register_component(input: TokenStream) -> TokenStream {
    derive_register_component::derive_register_component(input)
}

/// Derives `IntoElement` and `Element` for plot types that implement the `Plot` trait.
#[proc_macro_derive(IntoPlot)]
pub fn derive_into_plot(input: TokenStream) -> TokenStream {
    derive_into_plot::derive_into_plot(input)
}

/// Generate a custom icon enum by scanning a directory of SVG files.
///
/// Accepts an enum name, a path relative to the calling crate's `CARGO_MANIFEST_DIR`,
/// and optionally a list of additional derive traits.
///
/// # Example
///
/// ```ignore
/// icon_named!(IconName, "../assets/icons");
/// icon_named!(IconName, "../assets/icons", [Debug, Copy, PartialEq, Eq]);
/// ```
#[proc_macro]
pub fn icon_named(input: TokenStream) -> TokenStream {
    icon_named::icon_named(input)
}
