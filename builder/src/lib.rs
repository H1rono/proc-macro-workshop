use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput {
        attrs: _,
        vis: _,
        ident,
        generics: _,
        data,
    } = parse_macro_input!(input as DeriveInput);

    let builder_ident = format_ident!("{}Builder", ident);

    let Data::Struct(data) = data else {
        unimplemented!();
    };
    let Fields::Named(fields) = data.fields else {
        unimplemented!();
    };
    let named_fields: Vec<_> = fields
        .named
        .iter()
        .map(|n| {
            let ident = n.ident.as_ref().unwrap();
            let ty = &n.ty;
            (ident, ty)
        })
        .collect();
    let builder_fields = named_fields.iter().map(|(ident, ty)| {
        quote! {
            pub #ident: ::std::option::Option<#ty>
        }
    });
    let builder_methods = named_fields.iter().map(|(ident, ty)| {
        quote! {
            pub fn #ident(&mut self, #ident: #ty) -> &mut Self {
                self.#ident = ::std::option::Option::Some(#ident);
                self
            }
        }
    });

    quote! {
        #[derive(Default)]
        struct #builder_ident {
            #(#builder_fields),*
        }

        impl #builder_ident {
            #(#builder_methods)*
        }

        impl #ident {
            pub fn builder() -> #builder_ident {
                ::std::default::Default::default()
            }
        }
    }
    .into()
}
