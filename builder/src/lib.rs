mod field;

use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

use field::{Field, FieldTypeKind};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_builder(input) {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn derive_builder(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let DeriveInput {
        attrs: _,
        vis: _,
        ident,
        generics: _,
        data,
    } = &input;
    let builder_ident = format_ident!("{}Builder", ident);

    let Data::Struct(syn::DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = data
    else {
        return Err(syn::Error::new(
            input.span(),
            "`derive(Builder)` only accepts struct with named field",
        ));
    };
    let named_fields = fields
        .named
        .iter()
        .cloned()
        .map(Field::parse_field)
        .collect::<syn::Result<Vec<_>>>()?;
    let builder_fields = named_fields.iter().map(|Field { ident, ty, .. }| match ty {
        FieldTypeKind::OptionWrapped { ty: oty, .. } => quote! {
            pub #ident: ::std::option::Option<#oty>
        },
        FieldTypeKind::VecWrapped { ty: vty, .. } => quote! {
            pub #ident: ::std::vec::Vec<#vty>
        },
        FieldTypeKind::Raw(rty) => quote! {
            pub #ident: ::std::option::Option<#rty>
        },
    });

    let builder_methods = named_fields.iter().map(|Field { ident, ty, .. }| {
        let method = match ty {
            FieldTypeKind::OptionWrapped { ty: oty, .. } => quote! {
                pub fn #ident(&mut self, #ident: #oty) -> &mut Self {
                    self.#ident = ::std::option::Option::Some(#ident);
                    self
                }
            },
            FieldTypeKind::VecWrapped { ty: vty, .. } => quote! {
                pub fn #ident(&mut self, #ident: ::std::vec::Vec<#vty>) -> &mut Self {
                    self.#ident = #ident;
                    self
                }
            },
            FieldTypeKind::Raw(rty) => quote! {
                pub fn #ident(&mut self, #ident: #rty) -> &mut Self {
                    self.#ident = ::std::option::Option::Some(#ident);
                    self
                }
            },
        };
        (ident, method)
    });
    let builder_each_methods = named_fields.iter().filter_map(|field| {
        let Field {
            ident, ty, attrs, ..
        } = field;
        let field::FieldAttribute { each_val, .. } = attrs.as_ref()?;
        let FieldTypeKind::VecWrapped { ty, .. } = ty else {
            panic!(r#"`#[builder(each = "...")]` can only be used for `Vec<T>`"#);
        };
        let method = quote! {
            pub fn #each_val(&mut self, #each_val: #ty) -> &mut Self {
                self.#ident.push(#each_val);
                self
            }
        };
        Some((each_val, method))
    });
    let builder_methods = builder_methods
        .chain(builder_each_methods)
        .collect::<HashMap<_, _>>();
    let builder_methods = builder_methods.into_values();

    let build_method_fields = named_fields.iter().map(|Field { ident, ty, .. }| match ty {
        FieldTypeKind::OptionWrapped { .. } => quote! {
            #ident: self.#ident.take()
        },
        FieldTypeKind::VecWrapped { .. } => quote! {
            #ident: {
                let v = self.#ident.clone();
                self.#ident = vec![];
                v
            }
        },
        FieldTypeKind::Raw(_) => quote! {
            #ident: self.#ident.take()
                .ok_or(concat!("field ", stringify!(#ident), " is not set").to_string())?
        },
    });

    let code = quote! {
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
    };
    Ok(code)
}
