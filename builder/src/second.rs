use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Field, Fields, Ident, LitStr, Meta, Result, Type
};

pub fn expand(input: DeriveInput) -> Result<TokenStream2> {
    let vis = &input.vis;
    let input_ident = &input.ident;
    let builder_ident = Ident::new(&format!("{}Builder", input_ident), Span::call_site());

    let fields = match input.data {
        Data::Struct(s) => match s.fields {
            Fields::Named(fields) => fields.named,
            _ => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    };

    let builder_fields: Vec<_> = fields
        .iter()
        .map(BuilderField::try_from)
        .collect::<Result<_>>()?;

    let storage = make_storage(&builder_fields);
    let initializer = make_initializer(&builder_fields);
    let setters = make_setters(&builder_fields);
    let buildfn = make_buildfn(&input_ident, &builder_fields);

    Ok(quote! {
        #vis struct #builder_ident {
            #storage
        }

        impl #input_ident {
            #vis fn builder() -> #builder_ident {
                #builder_ident {
                    #initializer
                }
            }
        }

        impl #builder_ident {
            #setters
            #buildfn
        }
    })
}

struct BuilderField {
    ident: Ident,
    ty: FieldType,
}

enum FieldType {
    Plain(Type),
    Optional(Type),
    Repeated(Ident, Type),
}

use self::FieldType::*;

impl BuilderField {
    fn new(ident: Ident, ty: FieldType) -> Self {
        BuilderField { ident, ty }
    }

    fn try_from(field: &Field) -> Result<Self> {
        let mut each = None::<Ident>;
        let ident = field.ident.clone().unwrap();

        // 要求：
        //     1. 必要字段用 Option 包裹，在最后 build 的时候验证不为 None
        //     2. 可选字段本身就是 Option<T> 类型，不需要再用 Option 包裹
        //     3. Vec<T> 类型字段可用 #[builder(each = "...")] 指定添加一次一个的 setter 函数
        //        在其他类型上使用 each 在字段标识符位置报错
        // 为了标识字段属于哪一种情况，使用 FieldType 枚举 [Plain(1), Optional(2), Repeated(3)]
        if let Type::Path(ty) = &field.ty {
            // 情况 3 ---- Inert Attribute
            for attr in &field.attrs {
                if !attr.path().is_ident("builder") {
                    continue;
                }

                // 只允许 Vec<T> 类型字段
                if ty.path.segments.last().unwrap().ident != "Vec" {
                    return Err(Error::new_spanned(ident, r#"Only allowed to use `#[builder(each = "...")]` on the `Vec<T>` field."#));
                }

                let expected = r#"expected `builder(each = "...")`"#;
                let meta = match &attr.meta {
                    Meta::List(meta) => meta,       // 只能是 builder(...) 的格式
                    meta => return Err(Error::new_spanned(meta, expected)),
                };

                meta.parse_nested_meta(|nested| {
                    if nested.path.is_ident("each") {
                        let lit: LitStr = nested.value()?.parse()?;     // 注意这里需要先解析为 LitStr，即字面常量
                        each = Some(lit.parse()?);                      // 再将 LitStr 解析为 Ident，不能跳过 LitStr 直接解析为 Ident
                        Ok(())
                    } else {
                        Err(Error::new_spanned(meta, expected))
                    }
                })?;
            }

            if let Some(each) = each {
                return Ok(BuilderField::new(ident, Repeated(each, field.ty.clone())));
            }

            // 情况 2 ---- Option
            if ty.path.segments.last().unwrap().ident == "Option" {
                return Ok(BuilderField::new(ident, Optional(field.ty.clone())));
            }
        }

        // 其它都算是情况 1
        Ok(BuilderField::new(ident, Plain(field.ty.clone())))
    }
}

fn make_storage(fields: &[BuilderField]) -> TokenStream2 {
    fields
        .iter()
        .map(|field| {
            let ident = &field.ident;
            let storage = match &field.ty {
                Plain(ty) => quote! { std::option::Option<#ty> },
                Optional(ty) | Repeated(_, ty) => quote! { #ty },
            };
            quote! {
                #ident: #storage,
            }
        })
        .collect()
}

fn make_initializer(fields: &[BuilderField]) -> TokenStream2 {
    fields
        .iter()
        .map(|field| {
            let ident = &field.ident;
            let init = match &field.ty {
                Repeated(_, ty) => quote!(<#ty>::new()),
                Plain(_) | Optional(_) => quote!(std::option::Option::None),
            };
            quote! {
                #ident: #init,
            }
        })
        .collect()
}

fn make_setters(fields: &[BuilderField]) -> TokenStream2 {
    fields
        .iter()
        .map(|field| {
            let ident = &field.ident;
            let plain_store = quote!(self.#ident = std::option::Option::Some(#ident));
            let repeated_store = |each| quote!(self.#ident.push(#each));
            let inner = |ty| quote!(<#ty as std::iter::IntoIterator>::Item);
            let (ident, arg, store) = match &field.ty {
                Plain(ty) => (ident, quote!(#ty), plain_store),
                Optional(ty) => (ident, inner(ty), plain_store),
                Repeated(each, ty) => (each, inner(ty), repeated_store(each)),
            };
            quote! {
                fn #ident(&mut self, #ident: #arg) -> &mut Self {
                    #store;
                    self
                }
            }
        })
        .collect()
}

fn make_buildfn(input_ident: &Ident, fields: &[BuilderField]) -> TokenStream2 {
    let required_field_checks: TokenStream2 = fields
        .iter()
        .filter_map(|field| {
            let ident = &field.ident;
            let error = format!("value is not set: {}", ident);
            match &field.ty {
                Plain(_) => Some(quote! {
                    let #ident = self.#ident.take().ok_or_else(|| {
                        std::boxed::Box::<dyn std::error::Error>::from(#error)
                    })?;
                }),
                Optional(_) | Repeated(..) => None,
            }
        })
        .collect();
    
    let field_assignments: TokenStream2 = fields
        .iter()
        .map(|field| {
            let ident = &field.ident;
            let expr = match &field.ty {
                Plain(_) => quote!(#ident),
                Optional(_) => quote!(self.#ident.take()),
                Repeated(..) => quote!(std::mem::take(&mut self.#ident)),
            };

            quote!(#ident: #expr,)
        })
        .collect();

    quote! {
        fn build(&mut self) -> std::result::Result<#input_ident, std::boxed::Box<dyn std::error::Error>> {
            #required_field_checks
            Ok(#input_ident {
                #field_assignments
            })
        }
    }
}
