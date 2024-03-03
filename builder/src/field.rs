#![allow(unused)]

use syn::{spanned::Spanned, Attribute, Error, Ident, Type};

#[derive(Clone)]
pub struct Field {
    pub attrs: Option<FieldAttribute>,
    pub vis: syn::Visibility,
    pub ident: Ident,
    pub colon_token: syn::Token![:],
    pub ty: FieldTypeKind,
    pub span: proc_macro2::Span,
}

#[derive(Clone)]
pub enum FieldTypeKind {
    OptionWrapped {
        option: Ident,
        angle_open: syn::Token![<],
        ty: Type,
        angle_close: syn::Token![>],
    },
    VecWrapped {
        vec: Ident,
        angle_open: syn::Token![<],
        ty: Type,
        angle_close: syn::Token![>],
    },
    Raw(Type),
}

#[derive(Clone)]
pub struct FieldAttribute {
    pub pound_token: syn::Token![#],
    pub bracket_token: syn::token::Bracket,
    pub builder: Ident,
    pub paren_token: syn::token::Paren,
    pub each_ident: Ident,
    pub each_eq: syn::Token![=],
    pub each_val: Ident,
}

#[derive(Clone)]
pub struct FieldAttrInner {
    pub key: Ident,
    pub eq: syn::Token![=],
    pub val: syn::LitStr,
}

impl Field {
    pub fn parse_field(field: syn::Field) -> syn::Result<Self> {
        let span = field.span();
        let syn::Field {
            mut attrs,
            vis,
            mutability: _,
            ident,
            colon_token,
            ty,
        } = field;
        let attrs = attrs
            .pop()
            .map(FieldAttribute::parse_attribute)
            .transpose()?;
        let ty = FieldTypeKind::parse(ty);
        Ok(Self {
            attrs,
            vis,
            ident: ident.unwrap(),
            colon_token: colon_token.unwrap(),
            ty,
            span,
        })
    }
}

impl FieldTypeKind {
    pub fn parse(ty: Type) -> Self {
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

        use syn::{punctuated::Pair, GenericArgument, PathArguments};

        filter_try!(let Type::Path(mut p) = ty.clone());
        filter_try!(if p.qself.is_some() || p.path.leading_colon.is_some());
        filter_try!(let Some(Pair::End(seg)) = p.path.segments.pop());
        if seg.ident == "Option" {
            filter_try!(let PathArguments::AngleBracketed(mut seg_args) = seg.arguments);
            filter_try!(if seg_args.args.len() != 1);
            filter_try!(let Some(Pair::End(GenericArgument::Type(arg_ty))) = seg_args.args.pop());
            return Self::OptionWrapped {
                option: seg.ident,
                angle_open: seg_args.lt_token,
                ty: arg_ty,
                angle_close: seg_args.gt_token,
            };
        }
        if seg.ident == "Vec" {
            filter_try!(let PathArguments::AngleBracketed(mut seg_args) = seg.arguments);
            filter_try!(if seg_args.args.len() != 1);
            filter_try!(let Some(Pair::End(GenericArgument::Type(arg_ty))) = seg_args.args.pop());
            return Self::VecWrapped {
                vec: seg.ident,
                angle_open: seg_args.lt_token,
                ty: arg_ty,
                angle_close: seg_args.gt_token,
            };
        }
        Self::Raw(ty)
    }

    pub fn as_inner(&self) -> &Type {
        match self {
            Self::OptionWrapped { ty: oty, .. } => oty,
            Self::VecWrapped { ty: vty, .. } => vty,
            Self::Raw(rty) => rty,
        }
    }

    pub fn into_inner(self) -> Type {
        match self {
            Self::OptionWrapped { ty: oty, .. } => oty,
            Self::VecWrapped { ty: vty, .. } => vty,
            Self::Raw(rty) => rty,
        }
    }

    pub fn is_raw(&self) -> bool {
        matches!(self, Self::Raw(_))
    }

    pub fn is_vec_wrapped(&self) -> bool {
        matches!(self, Self::VecWrapped { .. })
    }

    pub fn is_opt_wrapped(&self) -> bool {
        matches!(self, Self::OptionWrapped { .. })
    }
}

impl FieldAttribute {
    pub fn parse_attribute(attribute: Attribute) -> syn::Result<Self> {
        let span = attribute.meta.span();
        let err = || Error::new(span, r#"expected `builder(each = "...")`"#);
        if let syn::AttrStyle::Inner(bang) = &attribute.style {
            return Err(Error::new(
                attribute.span(),
                "`builder` attribute only accepts outer one (the one without `!`)",
            ));
        }
        let syn::Meta::List(meta_list) = attribute.meta else {
            return Err(err());
        };
        let syn::MacroDelimiter::Paren(paren_token) = meta_list.delimiter else {
            return Err(err());
        };
        let builder = meta_list.path.get_ident().cloned().ok_or(err())?;
        if builder == "builder" {
            return Err(err());
        }
        let FieldAttrInner {
            key: each_ident,
            eq: each_eq,
            val: each_val,
        } = syn::parse2(meta_list.tokens)?;
        if each_ident != "each" {
            return Err(err());
        }
        let each_val = syn::parse_str(&each_val.value())?;
        Ok(Self {
            pound_token: attribute.pound_token,
            bracket_token: attribute.bracket_token,
            builder,
            paren_token,
            each_ident,
            each_eq,
            each_val,
        })
    }
}

impl syn::parse::Parse for FieldAttrInner {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        let eq: syn::Token![=] = input.parse()?;
        let val: syn::LitStr = input.parse()?;
        Ok(Self { key, eq, val })
    }
}
