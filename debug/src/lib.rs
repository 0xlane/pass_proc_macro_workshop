use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::collections::HashSet as Set;
use syn::{
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    visit::{self, Visit},
    Attribute, Data, DeriveInput, Error, Expr, ExprLit, Field, Fields, FieldsNamed, Generics,
    Ident, Lit, LitStr, Meta, Result, Token, TypePath, WherePredicate,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // eprintln!("{:#?}", input);
    let expand = expand(input).unwrap_or_else(|e| e.to_compile_error());
    TokenStream::from(expand)
}

fn expand(input: DeriveInput) -> Result<TokenStream2> {
    let input_ident = input.ident;
    let input_ident_name = input_ident.to_string();

    let fields = match input.data {
        Data::Struct(s) => match s.fields {
            Fields::Named(fields) => fields,
            _ => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    };

    let mut generics = input.generics;
    generics.make_where_clause();
    let mut where_clause = generics.where_clause.take().unwrap();
    match custom_clauses(&input.attrs)? {
        Some(custom_clauses) => {
            where_clause.predicates.extend(custom_clauses);
        }
        None => {
            let used_type_params = GenericVisitor::get_used_type_params(&generics, &fields);
            for ty_param in used_type_params {
                where_clause
                    .predicates
                    .push(parse_quote!(#ty_param: std::fmt::Debug));
            }
        }
    }

    let (impl_generics, ty_generics, _) = generics.split_for_impl();

    let debug_fields: Vec<_> = fields
        .named
        .iter()
        .map(DebugField::try_from)
        .collect::<Result<_>>()?;
    let field_calls = make_field_calls(&debug_fields);

    Ok(quote! {
        impl #impl_generics std::fmt::Debug for #input_ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(#input_ident_name)
                    #field_calls
                    .finish()
            }
        }
    })
}

struct DebugField {
    ident: Ident,
    fmt_arg: Option<String>,
}

impl DebugField {
    fn new(ident: Ident, fmt_arg: Option<String>) -> Self {
        DebugField { ident, fmt_arg }
    }

    fn try_from(field: &Field) -> Result<Self> {
        let ident = field.ident.clone().unwrap();
        let mut fmt_arg = None::<String>;

        for attr in &field.attrs {
            if !attr.path().is_ident("debug") {
                continue;
            }

            let expected = r#"expected `debug = "..."`"#;
            let meta = match &attr.meta {
                Meta::NameValue(meta) => meta,
                meta => return Err(Error::new_spanned(meta, expected)),
            };

            if !meta.path.is_ident("debug") {
                return Err(Error::new_spanned(meta, expected));
            }

            if let Expr::Lit(ExprLit {
                lit: Lit::Str(ref lit),
                ..
            }) = &meta.value
            {
                fmt_arg = Some(lit.value());
            } else {
                return Err(Error::new_spanned(meta, expected));
            }
        }

        Ok(DebugField::new(ident, fmt_arg))
    }
}

struct GenericVisitor<'ast> {
    _all_type_params: Vec<&'ast Ident>,
    all_used_type: Set<&'ast TypePath>,
}

impl<'ast> GenericVisitor<'ast> {
    fn get_used_type_params(
        generics: &'ast Generics,
        fields: &'ast FieldsNamed,
    ) -> Set<&'ast TypePath> {
        let mut vistor = GenericVisitor {
            _all_type_params: generics.type_params().map(|param| &param.ident).collect(),
            all_used_type: Set::new(),
        };
        vistor.visit_fields_named(fields);
        vistor.all_used_type
    }
}

impl<'ast> Visit<'ast> for GenericVisitor<'ast> {
    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        let segments = &ty.path.segments;

        if self._all_type_params.contains(&&segments[0].ident) {
            self.all_used_type.insert(ty);
        }

        if segments.last().unwrap().ident != "PhantomData" {
            visit::visit_type_path(self, ty);
        }
    }
}

fn custom_clauses(attrs: &[Attribute]) -> Result<Option<Vec<WherePredicate>>> {
    let mut where_clauses = None::<Vec<WherePredicate>>;

    for attr in attrs {
        if !attr.path().is_ident("debug") {
            continue;
        }

        let expected = r#"expected `debug(bound = "...")`"#;
        let meta = match &attr.meta {
            Meta::List(meta) => meta,
            _ => return Err(Error::new_spanned(attr, expected)),
        };

        meta.parse_nested_meta(|nested| {
            if !nested.path.is_ident("bound") {
                return Err(Error::new_spanned(attr, expected));
            }

            let lit: LitStr = nested.value()?.parse()?;
            let custom_clauses = lit.parse_with(Punctuated::<_, Token![,]>::parse_terminated)?;
            match where_clauses.as_mut() {
                Some(where_clauses) => where_clauses.extend(custom_clauses),
                None => where_clauses = Some(custom_clauses.into_iter().collect()),
            }

            Ok(())
        })?;
    }

    Ok(where_clauses)
}

fn make_field_calls(fields: &[DebugField]) -> TokenStream2 {
    fields
        .iter()
        .map(|field| {
            let ident = &field.ident;
            let ident_name = ident.to_string();

            match &field.fmt_arg {
                Some(fmt_arg) => {
                    quote!(.field(#ident_name, &format_args!(#fmt_arg, self.#ident)))
                }
                None => quote!(.field(#ident_name, &self.#ident)),
            }
        })
        .collect()
}
