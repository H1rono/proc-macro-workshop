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
    let build_method_fields = named_fields.iter().map(|(ident, _)| {
        quote! {
            #ident: self.#ident.take()
                .ok_or(concat!("field ", stringify!(#ident), " is not set").to_string())?
        }
    });

    quote! {
        #[derive(Default)]
        struct #builder_ident {
            #(#builder_fields),*
        }

        impl #builder_ident {
            #(#builder_methods)*

            pub fn build(&mut self) -> ::std::result::Result<
                #ident,
                ::std::boxed::Box<dyn ::std::error::Error>
            > {
                ::std::result::Result::Ok(#ident {
                    #(#build_method_fields),*
                })
            }
        }

        impl #ident {
            pub fn builder() -> #builder_ident {
                ::std::default::Default::default()
            }
        }
    }
    .into()
}
