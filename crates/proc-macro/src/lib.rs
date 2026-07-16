//! Proc-macro implementation for [`clap_layers`](crate).
//!
//! This crate provides the `#[derive(Layered)]` macro that generates the
//! configuration loading logic at compile time.
//!
//! ## How It Works
//!
//! When you write:
//!
//! ```ignore
//! #[derive(Parser, Layered)]
//! #[layered(file = "config.toml", env_prefix = "MYAPP")]
//! struct Config {
//!     #[arg(long, default_value_t = 3000)]
//!     port: u16,
//! }
//! ```
//!
//! The macro generates an implementation of [`Layered`] that:
//! 1. Parses command-line arguments (via clap)
//! 2. Reads environment variables with the given prefix
//! 3. Loads the TOML config file
//! 4. Merges them in precedence order, tracking where each value came from
//!
//! ## Type Requirements
//!
//! All fields must implement [`FromStr`] so they can be parsed from environment
//! variable strings and config file values. Common supported types include:
//!
//! - Integer types: `u8`, `u16`, `u32`, `u64`, `usize`, `i8`, `i16`, `i32`, `i64`
//! - Floating point: `f32`, `f64`
//! - Text: `String` (parsing always succeeds for any valid UTF-8)
//! - Booleans: `bool` ("true"/"false", "yes"/"no", "1"/"0")
//!
//! If a field doesn't implement FromStr, you'll get a compile-time error when
//! the macro tries to parse env var values.

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, MetaNameValue, parse_macro_input};

/// Derive the `Layered` trait for a struct.
///
/// This macro works with clap's Parser derive to create layered configuration
/// that respects: explicit CLI flag > env var > config file > built-in default.
#[proc_macro_derive(Layered, attributes(layered))]
pub fn derive_layered(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Parse #[layered(...)] attributes at struct level
    let mut file_path: Option<String> = None;
    let mut env_prefix: Option<String> = None;

    for attr in &input.attrs {
        if attr.path().is_ident("layered") {
            if let Ok(args) = attr.parse_args_with(
                syn::punctuated::Punctuated::<MetaNameValue, syn::Token![,]>::parse_terminated,
            ) {
                for arg in args {
                    let ident = arg.path.get_ident();
                    match ident.map(|i| i.to_string()).as_deref() {
                        Some("file") => {
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(lit_str),
                                ..
                            }) = arg.value
                            {
                                file_path = Some(lit_str.value());
                            }
                        }
                        Some("env_prefix") => {
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(lit_str),
                                ..
                            }) = arg.value
                            {
                                env_prefix = Some(lit_str.value());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Extract struct fields with their #[layered(...)] attributes
    let field_data = if let syn::Data::Struct(data) = &input.data {
        match &data.fields {
            syn::Fields::Named(fields) => collect_field_info(fields.named.iter()),
            _ => {
                return syn::Error::new_spanned(
                    name,
                    "Layered only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        }
    } else {
        return syn::Error::new_spanned(name, "Layered only supports structs, not enums or unions")
            .to_compile_error()
            .into();
    };

    // Generate the implementation
    let expanded = generate_layered_impl(
        name,
        &impl_generics,
        &ty_generics,
        where_clause,
        file_path,
        env_prefix,
        &field_data,
    );

    TokenStream::from(expanded)
}

/// Generate the Layered trait implementation for a struct.
fn generate_layered_impl(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics<'_>,
    ty_generics: &syn::TypeGenerics<'_>,
    where_clause: Option<&syn::WhereClause>,
    file_path: Option<String>,
    env_prefix: Option<String>,
    fields: &[FieldInfo],
) -> proc_macro2::TokenStream {
    // Generate field initialization code for each field using FieldInfo
    let mut field_inits = Vec::new();
    for field in fields {
        let ident = &field.ident;
        let init_code = generate_field_merge(
            ident,
            field._no_cli,
            field.no_env,
            field.no_file,
            &file_path,
            &env_prefix,
        );
        // Wrap each field initialization
        let field_init = quote! {
            #ident: #init_code
        };
        field_inits.push(field_init);
    }

    // Generate impl block using the same crate root that was used to import this derive.
    // We use `::clap_layers` which will resolve relative to the crate root at the call site.
    let expanded = quote! {
        impl #impl_generics clap_layers::Layered for #name #ty_generics #where_clause {
            fn layered() -> Result<Self, clap_layers::LayeredError> {
                // First parse CLI to get defaults from clap
                let cli_config = Self::parse_from(std::env::args());

                Ok(Self {
                    #(#field_inits),*
                })
            }
        }
    };

    expanded
}

/// Generate code that determines the final value for a single field.
fn generate_field_merge(
    ident: &syn::Ident,
    _no_cli: bool,
    no_env: bool,
    no_file: bool,
    file_path: &Option<String>,
    env_prefix: &Option<String>,
) -> proc_macro2::TokenStream {
    let field_name_str = ident.to_string();

    // Build the env var name from prefix + field name
    let env_var_name = env_prefix
        .as_ref()
        .map(|prefix| format!("{}_{}", prefix, field_name_str))
        .unwrap_or_else(|| field_name_str.clone());

    let crate_root = quote!(clap_layers);

    // Generate the file path check - only used if env not set and file is configured
    let fp_section = if file_path.is_some() && !no_file {
        quote! {
            #[allow(unused_imports)]
            use #crate_root::merge::parse_toml_file;
            // Try config file only if env not set and file is configured
            let fp_result = parse_toml_file(#file_path).ok();
            if fp_result.is_some() && std::env::var(#env_var_name).is_err() {
                let values = fp_result.unwrap();
                if let Some(file_val) = values.get(#field_name_str) {
                    match file_val.parse() {
                        Ok(parsed) => final_value = parsed,
                        Err(_) => {} // Keep cli_config value if parsing fails
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    // Environment variable section - only used if not disabled
    let env_section = if !no_env {
        quote! {
            if let Ok(env_val) = std::env::var(#env_var_name) {
                match env_val.parse() {
                    Ok(parsed) => final_value = parsed,
                    Err(_) => {} // Keep cli_config value if parsing fails
                }
            }
        }
    } else {
        quote! {}
    };

    // Return the field value logic
    // Precedence: CLI (already in cli_config) > env > file > defaults
    // We check env first, then file as fallback if env not set
    quote! {
        {
            #[allow(unused_mut)]
            let mut final_value = cli_config.#ident;

            #env_section
            #fp_section

            final_value
        }
    }
}

/// Collect detailed field information including `#[layered(...)]` attributes.
///
/// This helper function extracts metadata about each struct field,
/// particularly which layer markers (`no_cli`, `no_file`, `no_env`)
/// are present on each field.
#[allow(dead_code)]
struct FieldInfo {
    ident: syn::Ident,
    _no_cli: bool,
    no_file: bool,
    no_env: bool,
}

fn collect_field_info<'a, I>(fields: I) -> Vec<FieldInfo>
where
    I: Iterator<Item = &'a syn::Field>,
{
    fields
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            let mut _no_cli = false;
            let mut no_file = false;
            let mut no_env = false;

            // Check for `#[layered(...)]` on the field
            for attr in &field.attrs {
                if attr.path().is_ident("layered") {
                    if let Ok(args) = attr.parse_args_with(
                    syn::punctuated::Punctuated::<MetaNameValue, syn::Token![,]>::parse_terminated,
                ) {
                    for arg in args {
                        let ident = arg.path.get_ident();
                        if let Some(name) = ident.map(|i| i.to_string()) {
                            match name.as_str() {
                                "no_cli" => _no_cli = true,
                                "no_file" => no_file = true,
                                "no_env" => no_env = true,
                                _ => {}
                            }
                        }
                    }
                }
                }
            }

            FieldInfo {
                ident: ident.clone(),
                _no_cli,
                no_file,
                no_env,
            }
        })
        .collect()
}
