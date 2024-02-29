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
    let builder_fields = fields.named.into_iter().map(|n| {
        let ident = n.ident.unwrap();
        let ty = n.ty;
        quote! {
            pub #ident: ::std::option::Option<#ty>
        }
    });

    quote! {
        #[derive(Default)]
        struct #builder_ident {
            #(#builder_fields),*
        }

        impl #ident {
            pub fn builder() -> #builder_ident {
                ::std::default::Default::default()
            }
        }
    }
    .into()
}
