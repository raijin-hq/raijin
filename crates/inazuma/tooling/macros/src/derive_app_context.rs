use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use crate::get_simple_attribute_field;

pub fn derive_app_context(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let Some(app_variable) = get_simple_attribute_field(&ast, "app") else {
        return quote! {
            compile_error!("Derive must have an #[app] attribute to detect the &mut App field");
        }
        .into();
    };

    let type_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();

    let r#gen = quote! {
        impl #impl_generics inazuma::AppContext for #type_name #type_generics
        #where_clause
        {
            fn new<T: 'static>(
                &mut self,
                build_entity: impl FnOnce(&mut inazuma::Context<'_, T>) -> T,
            ) -> inazuma::Entity<T> {
                self.#app_variable.new(build_entity)
            }

            fn reserve_entity<T: 'static>(&mut self) -> inazuma::Reservation<T> {
                self.#app_variable.reserve_entity()
            }

            fn insert_entity<T: 'static>(
                &mut self,
                reservation: inazuma::Reservation<T>,
                build_entity: impl FnOnce(&mut inazuma::Context<'_, T>) -> T,
            ) -> inazuma::Entity<T> {
                self.#app_variable.insert_entity(reservation, build_entity)
            }

            fn update_entity<T, R>(
                &mut self,
                handle: &inazuma::Entity<T>,
                update: impl FnOnce(&mut T, &mut inazuma::Context<'_, T>) -> R,
            ) -> R
            where
                T: 'static,
            {
                self.#app_variable.update_entity(handle, update)
            }

            fn as_mut<'y, 'z, T>(
                &'y mut self,
                handle: &'z inazuma::Entity<T>,
            ) -> inazuma::GpuiBorrow<'y, T>
            where
                T: 'static,
            {
                self.#app_variable.as_mut(handle)
            }

            fn read_entity<T, R>(
                &self,
                handle: &inazuma::Entity<T>,
                read: impl FnOnce(&T, &inazuma::App) -> R,
            ) -> R
            where
                T: 'static,
            {
                self.#app_variable.read_entity(handle, read)
            }

            fn update_window<T, F>(&mut self, window: inazuma::AnyWindowHandle, f: F) -> inazuma::Result<T>
            where
                F: FnOnce(inazuma::AnyView, &mut inazuma::Window, &mut inazuma::App) -> T,
            {
                self.#app_variable.update_window(window, f)
            }

            fn read_window<T, R>(
                &self,
                window: &inazuma::WindowHandle<T>,
                read: impl FnOnce(inazuma::Entity<T>, &inazuma::App) -> R,
            ) -> inazuma::Result<R>
            where
                T: 'static,
            {
                self.#app_variable.read_window(window, read)
            }

            fn background_spawn<R>(&self, future: impl std::future::Future<Output = R> + Send + 'static) -> inazuma::Task<R>
            where
                R: Send + 'static,
            {
                self.#app_variable.background_spawn(future)
            }

            fn read_global<G, R>(&self, callback: impl FnOnce(&G, &inazuma::App) -> R) -> R
            where
                G: inazuma::Global,
            {
                self.#app_variable.read_global(callback)
            }
        }
    };

    r#gen.into()
}
