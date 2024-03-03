use syn::Type;

#[derive(Clone, Copy)]
pub enum FieldTypeInfo<'a> {
    OptionWrapped(&'a Type),
    VecWrapped(&'a Type),
    Raw(&'a Type),
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

        use syn::{GenericArgument, PathArguments};

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
