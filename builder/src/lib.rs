use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, token::Comma, DataStruct, DeriveInput, Field, GenericArgument, Ident, LitStr, Type};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // eprintln!("{:#?}", input);
    TokenStream::from(
        match do_expand(&input) {
            Ok(token_stream) => token_stream,
            Err(e) => e.to_compile_error()
        }
    )
}

fn do_expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let original_ident = &input.ident;
    let builder_ident_literal = format!("{}Builder", original_ident.to_string());
    let builder_ident = Ident::new(&builder_ident_literal, original_ident.span());

    let fields = get_fields_from_input(input)?;
    let field_defines = generate_builder_field_defines(fields)?;
    let field_inits = generate_builder_field_inits(fields)?;
    let field_setter_funtions = generate_builder_setter_functions(fields)?;
    let build_function = generate_builder_build_function(fields, original_ident)?;

    let ret = quote! {
        pub struct #builder_ident {
            #field_defines
        }

        impl #builder_ident {
            #field_setter_funtions

            #build_function
        }

        impl #original_ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #field_inits
                }
            }
        }
    };

    Ok(ret)
}

fn get_fields_from_input(input: &DeriveInput) -> syn::Result<&Punctuated<Field, Comma>> {
    if let syn::Data::Struct(
        DataStruct {
            fields: syn::Fields::Named(
                syn::FieldsNamed { ref named, .. }
            ),
            ..
        }
    ) = input.data {
        Ok(named)
    } else {
        Err(syn::Error::new_spanned(input, "Supported only struct."))
    }
}

fn generate_builder_field_defines(fields: &Punctuated<Field, Comma>) -> syn::Result<proc_macro2::TokenStream> {
    let mut ret = Ok(());
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields.iter()
        .map(|f| {
            if let Some(tn) = get_direct_type_name(&f.ty) {
                if tn == "Option" {
                    return get_inner_type(&f.ty).unwrap().first().unwrap().to_owned();
                } else if tn == "Vec" {
                    match parse_user_specified_iden_for_vec(f) {
                        Ok(user_ident) => {
                            eprintln!("{:#?}", user_ident);
                        },
                        Err(e) => {ret = Err(e);}
                    };
                }
            }
            &f.ty
        }).collect();

    Ok(quote! {
        #(#idents: std::option::Option<#types>),*
    })
}

fn generate_builder_field_inits(fields: &Punctuated<Field, Comma>) -> syn::Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();

    Ok(quote! {
        #(#idents: std::option::Option::None),*
    })
}

fn generate_builder_setter_functions(fields: &Punctuated<Field, Comma>) -> syn::Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields.iter()
        .map(|f| {
            if let Some(tn) = get_direct_type_name(&f.ty) {
                if tn == "Option" {
                    return get_inner_type(&f.ty).unwrap().first().unwrap().to_owned();
                }
            }
            &f.ty
        }).collect();

    Ok(quote! {
        #(
            pub fn #idents(&mut self, #idents: #types) -> &mut Self {
                self.#idents = std::option::Option::Some(#idents);
                self
            }
        )*
    })
}

fn generate_builder_build_function(fields: &Punctuated<Field, Comma>, original_ident: &Ident) -> syn::Result<proc_macro2::TokenStream> {
    let option_fields: Vec<_> = fields.iter()
        .filter(|f| {
            if let Some(tn) = get_direct_type_name(&f.ty) {
                tn == "Option"
            } else {
                true
            }
        }).collect();
    let other_fields: Vec<_> = fields.iter()
        .filter(|f| !option_fields.contains(f))
        .collect();
    let option_idents: Vec<_> = option_fields.iter().map(|f| &f.ident).collect();
    let other_idents: Vec<_> = other_fields.iter().map(|f| &f.ident).collect();

    Ok(quote! {
        pub fn build(&self) -> std::result::Result<#original_ident, std::boxed::Box<dyn std::error::Error>> {
            #(
                if self.#other_idents.is_none() {
                    return std::result::Result::Err(
                        format!("{} field missing", stringify!(#other_idents)).into()
                    )
                }
            )*

            std::result::Result::Ok(
                #original_ident {
                    #(#other_idents: self.#other_idents.clone().unwrap(),)*

                    #(#option_idents: self.#option_idents.clone(),)*
                }
            )
        }
    })
}

fn get_inner_type(ty: &Type) -> Option<Vec<&Type>> {
    if let syn::Type::Path(
        syn::TypePath {
            path: syn::Path {
                ref segments,
                ..
            },
            ..
        }
    ) = ty {
        
        if let Some(seg) = segments.last() {
            if let syn::PathArguments::AngleBracketed(
                syn::AngleBracketedGenericArguments {
                    ref args,
                    ..
                }
            ) = seg.arguments {
                let generic_types: Vec<_> = args.iter()
                    .filter_map(|arg| match arg {
                        GenericArgument::Type(ty) => Some(ty),
                        _ => None
                    }).collect();
                
                if generic_types.len() > 0 { 
                    return Some(generic_types);
                }
            }
        }
    }

    None
}

// fn get_direct_type_path(ty: &Type) -> Option<String> {
//     if let syn::Type::Path(
//         syn::TypePath {
//             path: syn::Path {
//                 ref segments,
//                 ..
//             },
//             ..
//         }
//     ) = ty {
//         let path: Vec<_> = segments.iter().map(|s| s.ident.to_string()).collect();

//         return Some(path.join("::"))
//     }

//     None
// }

fn get_direct_type_name(ty: &Type) -> Option<String> {
    if let syn::Type::Path(
        syn::TypePath {
            path: syn::Path {
                ref segments,
                ..
            },
            ..
        }
    ) = ty {
        if let Some(seg) = segments.last() {
            return Some(seg.ident.to_string())
        }
    }

    None
}

// fn get_inner_type_name(ty: &Type) -> Option<Vec<String>> {
//     match get_inner_type(ty) {
//         Some(inner_types) => Some(
//             inner_types.iter()
//                 .filter_map(|it| get_direct_type_name(it))
//                 .collect()
//         ),
//         None => None
//     }
// }

fn parse_user_specified_iden_for_vec(field: &Field) -> syn::Result<Option<Ident>> {
    for attr in &field.attrs {
        if attr.path().is_ident("builder") {
            let mut ret = None;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("each") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    ret = Some(Ident::new(&s.value(), field.ident.clone().unwrap().span()));
                    Ok(())
                } else {
                    Err(meta.error("unsupported attribute param"))
                }
            })?;
            return Ok(ret);
        } else {
            return Err(syn::Error::new_spanned(field, "unsupported attribute"));
        }
    }

    Ok(None)
}
