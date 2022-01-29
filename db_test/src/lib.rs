use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned, FnArg, ItemFn, Pat, Signature, Type};

#[proc_macro_attribute]
/// Provides a [`rocket::local::asynchronous::Client`] and [`mongodb::Database`] to the function,
/// instruments it as a [`rocket::async_test`] and ensures that the [`mongodb::Database`] is
/// cleared WHETHER OR NOT the test completes by passing, failing or otherwise panicking.
///
/// If a panic occurs via a failed assertion or other unwinding panic, the [`mongodb::Database`] is
/// cleared, and the panic is "rethrown".
///
/// Note: this attribute requires that [`rocket::async_test`], `client_with_db` and `clear_db` are
/// in scope. Additionally, the `futures` crate must be included as a test dependency so we can use
/// [`FutureExt::catch_unwind`].
pub fn db_test(_: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_fn = parse_macro_input!(input as ItemFn);
    let sig = item_fn.sig.clone();
    let name = sig.ident.clone();
    if let Err(err) = check_sig(sig.clone()) {
        return err.into_compile_error().into();
    }
    let new_name = format_ident!("{}_test", name);
    item_fn.sig.ident = new_name.clone();
    quote! {
        #[rocket::async_test]
        async fn #name() {
            let (client, db) = client_and_db().await;

            #item_fn

            // To avoid futures not being transferable across Unwind boundaries:
            // - See https://stackoverflow.com/a/66529014/13112498
            let client_mutex = std::sync::Mutex::new(client);
            let db_mutex = std::sync::Mutex::new(db.clone());

            let result = std::panic::catch_unwind(|| {
                let client = client_mutex.into_inner().unwrap();
                let db = db_mutex.into_inner().unwrap();

                let handle = tokio::runtime::Handle::current();

                let _ = handle.enter();

                futures::executor::block_on(#new_name(client, db));
            });

            db.drop(None).await.unwrap();

            if let Err(cause) = result {
                std::panic::panic_any(cause);
            }
        }
    }
    .into()
}

fn check_sig(sig: Signature) -> Result<(), syn::Error> {
    let inputs = sig.inputs;
    if inputs.len() != 2 {
        return Err(syn::Error::new(
            inputs.span(),
            "Arguments must be a `rocket::local::asynchronous::Client` and a `mongodb::Database`",
        ));
    }

    let mut has_client = false;
    let mut has_db = false;

    for input in &inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(_) = &*pat_type.pat {
                if let Type::Path(type_path) = &*pat_type.ty {
                    let path = &type_path.path;
                    let ty_ident = path
                        .get_ident()
                        .ok_or_else(|| {
                            syn::Error::new(
                                path.span(),
                                "Type must be a standalone type identifier",
                            )
                        })?
                        .to_string();
                    match ty_ident.as_str() {
                        "Client" => has_client = true,
                        "Database" => has_db = true,
                        _ => {
                            return Err(syn::Error::new(
                                ty_ident.span(),
                                "Expected either `Client` or `Database`",
                            ))
                        }
                    }
                } else {
                    return Err(syn::Error::new(
                        pat_type.ty.span(),
                        format!(
                            "Function argument type must be a type identifier, got {:?} instead",
                            pat_type.ty
                        ),
                    ));
                }
            } else {
                return Err(syn::Error::new(
                    pat_type.pat.span(),
                    "Function argument pattern must be an identifier",
                ));
            }
        } else {
            return Err(syn::Error::new(
                input.span(),
                format!("Function argument must not be a receiver type"),
            ));
        }
    }

    if has_client && has_db {
        Ok(())
    } else {
        Err(syn::Error::new(
            inputs.span(),
            "The tagged function must accept a `rocket::local::asynchronous::Client` and `mongodb::Database`",
        ))
    }
}
