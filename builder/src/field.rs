use syn::{Ident, Type};

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

#[allow(unused)]
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
