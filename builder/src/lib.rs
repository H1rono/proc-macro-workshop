use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, GenericArgument, Ident, PathArguments, Type,
};

#[derive(Clone, Copy)]
enum FieldTypeInfo<'a> {
    OptionWrapped(&'a Type),
    VecWrapped(&'a Type),
    Raw(&'a Type),
}

struct BuilderAttrArgs {
    each: Ident,
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_builder(input) {
        Ok(t) => t,
        Err(e) => e.to_compile_error().into(),
    }
}

fn derive_builder(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        attrs: _,
        vis: _,
        ident,
        generics: _,
        data,
    } = input;
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
            let ty = FieldTypeInfo::parse(&n.ty);
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
    let builder_fields = named_fields.iter().map(|&(ident, ty, _)| match ty {
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

    let builder_methods = named_fields.iter().map(|&(ident, ty, _)| {
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
    Ok(code.into())
}

#[allow(unused)]
impl<'a> FieldTypeInfo<'a> {
    pub fn parse(ty: &'a Type) -> Self {
        macro_rules! filter_try {
            (let $p:pat = $e:expr) => {
                let $p = $e else {
                    return Self::Raw(ty);
                };
            };
            (if $e:expr) => {
                if $e {
                    return Self::Raw(ty);
                }
            };
        }

        filter_try!(let Type::Path(p) = ty);
        filter_try!(if p.qself.is_some() || p.path.leading_colon.is_some());
        let Some(seg) = p.path.segments.first() else {
            return Self::Raw(ty);
        };
        if seg.ident == "Option" {
            let PathArguments::AngleBracketed(seg_args) = &seg.arguments else {
                return Self::Raw(ty);
            };
            if seg_args.args.len() != 1 {
                return Self::Raw(ty);
            }
            let Some(GenericArgument::Type(arg_ty)) = seg_args.args.first() else {
                return Self::Raw(ty);
            };
            return Self::OptionWrapped(arg_ty);
        }
        if seg.ident == "Vec" {
            let PathArguments::AngleBracketed(seg_args) = &seg.arguments else {
                return Self::Raw(ty);
            };
            if seg_args.args.len() != 1 {
                return Self::Raw(ty);
            }
            let Some(GenericArgument::Type(arg_ty)) = seg_args.args.first() else {
                return Self::Raw(ty);
            };
            return Self::VecWrapped(arg_ty);
        }
        Self::Raw(ty)
    }

    pub fn as_inner(&'a self) -> &'a Type {
        match self {
            Self::OptionWrapped(oty) => oty,
            Self::VecWrapped(vty) => vty,
            Self::Raw(rty) => rty,
        }
    }

    pub fn is_raw(&self) -> bool {
        matches!(self, Self::Raw(_))
    }

    pub fn is_vec_wrapped(&self) -> bool {
        matches!(self, Self::VecWrapped(_))
    }

    pub fn is_opt_wrapped(&self) -> bool {
        matches!(self, Self::OptionWrapped(_))
    }
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
