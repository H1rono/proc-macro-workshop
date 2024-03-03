mod field;

use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident};

use field::FieldTypeInfo;

struct BuilderAttrArgs {
    each: Ident,
}

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
    let named_fields: Vec<_> = fields
        .named
        .iter()
        .map(|n| {
            let ident = n.ident.as_ref().cloned().unwrap();
            let ty = FieldTypeInfo::parse(n.ty.clone());
            let args = n.attrs.iter().find_map(|a| {
                let id = a.path().get_ident()?;
                if id != "builder" {
                    return None;
                }
                let syn::Meta::List(meta_list) = &a.meta else {
                    return None;
                };
                let attr_args = meta_list.parse_args::<BuilderAttrArgs>().ok()?;
                Some(attr_args)
            });
            (ident, ty, args)
        })
        .collect();
    let builder_fields = named_fields.iter().map(|(ref ident, ty, _)| match ty {
        FieldTypeInfo::OptionWrapped(oty) => quote! {
            pub #ident: ::std::option::Option<#oty>
        },
        FieldTypeInfo::VecWrapped(vty) => quote! {
            pub #ident: ::std::vec::Vec<#vty>
        },
        FieldTypeInfo::Raw(rty) => quote! {
            pub #ident: ::std::option::Option<#rty>
        },
    });

    let builder_methods = named_fields.iter().map(|(ref ident, ty, _)| {
        let method = match ty {
            FieldTypeInfo::OptionWrapped(oty) => quote! {
                pub fn #ident(&mut self, #ident: #oty) -> &mut Self {
                    self.#ident = ::std::option::Option::Some(#ident);
                    self
                }
            },
            FieldTypeInfo::VecWrapped(vty) => quote! {
                pub fn #ident(&mut self, #ident: ::std::vec::Vec<#vty>) -> &mut Self {
                    self.#ident = #ident;
                    self
                }
            },
            FieldTypeInfo::Raw(rty) => quote! {
                pub fn #ident(&mut self, #ident: #rty) -> &mut Self {
                    self.#ident = ::std::option::Option::Some(#ident);
                    self
                }
            },
        };
        (ident, method)
    });
    let builder_each_methods = named_fields.iter().filter_map(|(ref ident, ty, args)| {
        let BuilderAttrArgs { each } = args.as_ref()?;
        let FieldTypeInfo::VecWrapped(ty) = ty else {
            panic!(r#"`#[builder(each = "...")]` can only be used for `Vec<T>`"#);
        };
        let method = quote! {
            pub fn #each(&mut self, #each: #ty) -> &mut Self {
                self.#ident.push(#each);
                self
            }
        };
        Some((each, method))
    });
    let builder_methods = builder_methods
        .chain(builder_each_methods)
        .collect::<HashMap<_, _>>();
    let builder_methods = builder_methods.into_values();

    let build_method_fields = named_fields.iter().map(|(ident, ty, _)| match ty {
        FieldTypeInfo::OptionWrapped(_) => quote! {
            #ident: self.#ident.take()
        },
        FieldTypeInfo::VecWrapped(_) => quote! {
            #ident: {
                let v = self.#ident.clone();
                self.#ident = vec![];
                v
            }
        },
        FieldTypeInfo::Raw(_) => quote! {
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

impl syn::parse::Parse for BuilderAttrArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let arg_id: Ident = input.parse()?;
        if arg_id != "each" {
            return Err(syn::Error::new(
                arg_id.span(),
                "unexpected argument, expected `each`",
            ));
        }
        let _: syn::Token![=] = input.parse()?;
        let arg_var: syn::LitStr = input.parse()?;
        let arg_var = Ident::new(&arg_var.value(), arg_var.span());
        Ok(Self { each: arg_var })
    }
}
