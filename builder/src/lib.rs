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

fn get_fields_from_input<'a>(input: &'a DeriveInput) -> syn::Result<&'a Punctuated<Field, Comma>> {
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
    let mut stream = proc_macro2::TokenStream::new();

    let field_names: Vec<_> = fields.iter()
        .filter_map(|f| f.ident.clone())
        .map(|ident| ident.to_string())
        .collect();

    for field in fields.iter() {
        if let Some(ident) = &field.ident {
            let mut ty = &field.ty;
            if let Some(type_name) = get_direct_type_name(ty) {
                if type_name == "Option" {
                    ty = get_inner_type(ty).unwrap().first().unwrap().to_owned();
                } else if type_name == "Vec" {
                    if let Some(user_ident) = parse_user_specified_iden_for_vec(field)? {
                        // let other_field_names: Vec<_> = field_names.iter().filter(|f| **f != ident.to_string()).collect();
                        // if !other_field_names.contains(&&user_ident.to_string()) {
                        if user_ident == *ident || (user_ident != *ident && !field_names.contains(&user_ident.to_string())) {
                            stream.extend(quote! {
                                #ident: #ty,
                            });
                            continue;
                        }
                    }
                }
            }
            
            stream.extend(quote! {
                #ident: std::option::Option<#ty>,
            });
        }
    }

    Ok(stream)
}

fn generate_builder_field_inits(fields: &Punctuated<Field, Comma>) -> syn::Result<proc_macro2::TokenStream> {
    let mut stream = proc_macro2::TokenStream::new();

    let field_names: Vec<_> = fields.iter()
        .filter_map(|f| f.ident.clone())
        .map(|ident| ident.to_string())
        .collect();

    for field in fields.iter() {
        if let Some(ident) = &field.ident {
            let ty = &field.ty;
            if let Some(type_name) = get_direct_type_name(ty) {
                if type_name == "Vec" {
                    if let Some(user_ident) = parse_user_specified_iden_for_vec(field)? {
                        // let other_field_names: Vec<_> = field_names.iter().filter(|f| **f != ident.to_string()).collect();
                        // if !other_field_names.contains(&&user_ident.to_string()) {
                        if user_ident == *ident || (user_ident != *ident && !field_names.contains(&user_ident.to_string())) {
                            stream.extend(quote! {
                                #ident: std::vec![],
                            });
                            continue;
                        }
                    }
                }
            }
            
            stream.extend(quote! {
                #ident: std::option::Option::None,
            });
        }
    }

    Ok(stream)
}

fn generate_builder_setter_functions(fields: &Punctuated<Field, Comma>) -> syn::Result<proc_macro2::TokenStream> {
    let mut stream = proc_macro2::TokenStream::new();

    let field_names: Vec<_> = fields.iter()
        .filter_map(|f| f.ident.clone())
        .map(|ident| ident.to_string())
        .collect();

    for field in fields.iter() {
        if let Some(ident) = &field.ident {
            let mut ty = &field.ty;
            if let Some(type_name) = get_direct_type_name(ty) {
                if type_name == "Option" {
                    ty = get_inner_type(ty).unwrap().first().unwrap().to_owned();
                } else if type_name == "Vec" {
                    if let Some(user_ident) = parse_user_specified_iden_for_vec(field)? {
                        // let other_field_names: Vec<_> = field_names.iter().filter(|f| **f != ident.to_string()).collect();
                        // if !other_field_names.contains(&&user_ident.to_string()) {
                        let inner_ty = get_inner_type(ty).unwrap().first().unwrap().to_owned();
                        if user_ident == *ident {
                            stream.extend(quote! {
                                pub fn #user_ident(&mut self, #user_ident: #inner_ty) -> &mut Self {
                                    self.#ident.push(#user_ident);
                                    self
                                }
                            });
                        } else if !field_names.contains(&user_ident.to_string()) {
                            stream.extend(quote! {
                                pub fn #ident(&mut self, #ident: #ty) -> &mut Self {
                                    self.#ident = #ident;
                                    self
                                }
        
                                pub fn #user_ident(&mut self, #user_ident: #inner_ty) -> &mut Self {
                                    self.#ident.push(#user_ident);
                                    self
                                }
                            });
                        }
                        continue;
                    }
                }
            }
            stream.extend(quote! {
                pub fn #ident(&mut self, #ident: #ty) -> &mut Self {
                    self.#ident = std::option::Option::Some(#ident);
                    self
                }
            });
        }
        
    }

    Ok(stream)
}

fn generate_builder_build_function(fields: &Punctuated<Field, Comma>, original_ident: &Ident) -> syn::Result<proc_macro2::TokenStream> {
    let mut check_stream = proc_macro2::TokenStream::new();
    let mut init_stream = proc_macro2::TokenStream::new();

    let field_names: Vec<_> = fields.iter()
        .filter_map(|f| f.ident.clone())
        .map(|ident| ident.to_string())
        .collect();

    for field in fields.iter() {
        if let Some(ident) = &field.ident {
            let ty = &field.ty;
            if let Some(type_name) = get_direct_type_name(ty) {
                if type_name == "Option" {
                    init_stream.extend(quote! {
                        #ident: self.#ident.clone(),
                    });
                    continue;
                } else if type_name == "Vec" {
                    if let Some(user_ident) = parse_user_specified_iden_for_vec(field)? {
                        // let other_field_names: Vec<_> = field_names.iter().filter(|f| **f != ident.to_string()).collect();
                        // if !other_field_names.contains(&&user_ident.to_string()) {
                        if user_ident == *ident || (user_ident != *ident && !field_names.contains(&user_ident.to_string())) {
                            init_stream.extend(quote! {
                                #ident: self.#ident.clone(),
                            });
                            continue;
                        }
                    }
                }
            }
            check_stream.extend(quote! {
                if self.#ident.is_none() {
                    return std::result::Result::Err(
                        format!("{} field missing", stringify!(#ident)).into()
                    )
                }
            });
            init_stream.extend(quote! {
                #ident: self.#ident.clone().unwrap(),
            });
        }
        
    }

    Ok(quote! {
        pub fn build(&self) -> std::result::Result<#original_ident, std::boxed::Box<dyn std::error::Error>> {
            #check_stream

            std::result::Result::Ok(
                #original_ident {
                    #init_stream
                }
            )
        }
    })
}

fn get_inner_type<'a>(ty: &'a Type) -> Option<Vec<&'a Type>> {
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
                    Err(syn::Error::new_spanned(&attr.meta, "expected `builder(each = \"...\")`"))
                }
            })?;
            return Ok(ret);
        } else {
            return Err(syn::Error::new_spanned(field, "unsupported attribute"));
        }
    }

    Ok(None)
}
