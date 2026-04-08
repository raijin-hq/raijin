use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn derive_register_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let register_fn_name = syn::Ident::new(
        &format!("__component_registry_internal_register_{}", name),
        name.span(),
    );
    let expanded = quote! {
        const _: () = {
            struct AssertComponent<T: inazuma_component_registry::Component>(::std::marker::PhantomData<T>);
            let _ = AssertComponent::<#name>(::std::marker::PhantomData);
        };

        #[allow(non_snake_case)]
        fn #register_fn_name() {
            inazuma_component_registry::register_component::<#name>();
        }

        inazuma_component_registry::__private::inventory::submit! {
            inazuma_component_registry::ComponentFn::new(#register_fn_name)
        }
    };
    expanded.into()
}
