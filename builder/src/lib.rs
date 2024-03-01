use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, GenericArgument, PathArguments, Type};

#[derive(Clone, Copy)]
enum FieldTypeInfo<'a> {
    OptionWrapped(&'a Type),
    Raw(&'a Type),
}

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
            let ty = FieldTypeInfo::parse(&n.ty);
            (ident, ty)
        })
        .collect();
    let builder_fields = named_fields.iter().map(|&(ident, ty)| {
        let ty = ty.as_inner();
        quote! {
            pub #ident: ::std::option::Option<#ty>
        }
    });
    let builder_methods = named_fields.iter().map(|&(ident, ty)| {
        let ty = ty.as_inner();
        quote! {
            pub fn #ident(&mut self, #ident: #ty) -> &mut Self {
                self.#ident = ::std::option::Option::Some(#ident);
                self
            }
        }
    });
    let build_method_fields = named_fields.iter().map(|(ident, ty)| match ty {
        FieldTypeInfo::OptionWrapped(_) => quote! {
            #ident: self.#ident.take()
        },
        FieldTypeInfo::Raw(_) => quote! {
            #ident: self.#ident.take()
                .ok_or(concat!("field ", stringify!(#ident), " is not set").to_string())?
        },
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

#[allow(unused)]
impl<'a> FieldTypeInfo<'a> {
    pub fn parse(ty: &'a Type) -> Self {
        let Type::Path(p) = ty else {
            return Self::Raw(ty);
        };
        if p.qself.is_some() || p.path.leading_colon.is_some() {
            return Self::Raw(ty);
        }
        let Some(seg) = p.path.segments.first() else {
            return Self::Raw(ty);
        };
        if &seg.ident.to_string() != "Option" {
            return Self::Raw(ty);
        }
        let PathArguments::AngleBracketed(seg_args) = &seg.arguments else {
            return Self::Raw(ty);
        };
        if seg_args.args.len() != 1 {
            return Self::Raw(ty);
        }
        let Some(GenericArgument::Type(arg_ty)) = seg_args.args.first() else {
            return Self::Raw(ty);
        };
        Self::OptionWrapped(arg_ty)
    }

    pub fn as_inner(&'a self) -> &'a Type {
        match self {
            Self::OptionWrapped(oty) => oty,
            Self::Raw(rty) => rty,
        }
    }

    pub fn is_raw(&self) -> bool {
        matches!(self, Self::Raw(_))
    }

    pub fn is_opt_wrapped(&self) -> bool {
        matches!(self, Self::OptionWrapped(_))
    }
}
