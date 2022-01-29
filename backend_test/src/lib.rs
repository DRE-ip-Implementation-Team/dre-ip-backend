use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Comma, FnArg, ItemFn, Pat,
    Signature, Type,
};

#[proc_macro_attribute]
/// Provides a [`rocket::local::asynchronous::Client`] and [`mongodb::Database`] to the function,
/// instruments it as a [`rocket::async_test`] and ensures that the [`mongodb::Database`] is
/// cleared WHETHER OR NOT the test completes by passing, failing or otherwise panicking.
///
/// If a panic occurs via a failed assertion or other unwinding panic, the [`mongodb::Database`] is
/// cleared, and the panic is "rethrown".
pub fn backend_test(_: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_fn = parse_macro_input!(input as ItemFn);

    // Reject invalid function signatures
    let args_used = match check_sig(item_fn.sig.clone()) {
        Ok(args) => args,
        Err(err) => {
            return err.into_compile_error().into();
        }
    };

    // Rename the future so the test can have its original name
    let name = item_fn.sig.ident;
    let new_name = format_ident!("{}_fut", name);
    item_fn.sig.ident = new_name.clone();

    let test_args = args_used.into_iter().collect::<Punctuated<_, Comma>>();

    quote! {
        #[rocket::async_test]
        async fn #name() {
            let db_client = crate::db_client().await;
            let rocket_client = rocket::local::asynchronous::Client::tracked(crate::rocket_for_db_client(db_client.clone()).await).await.unwrap();
            let db = db_client.database(crate::DATABASE);

            #item_fn

            // `Mutex<T>: UnwindSafe` circumvents `T: !UnwindSafe`:
            // - See https://stackoverflow.com/a/66529014/13112498
            let client_mutex = std::sync::Mutex::new(rocket_client);
            let db_mutex = std::sync::Mutex::new(db.clone());

            let result = std::panic::catch_unwind(|| {
                // Transfer mutexes across the unwind boundary
                // Unwraps are valid as no poisoning occurs: mutexes are not given no other threads
                let rocket_client = client_mutex.into_inner().unwrap();
                let db = db_mutex.into_inner().unwrap();

                // Manually run future inside
                let handle = rocket::tokio::runtime::Handle::current();
                let _ = handle.enter();
                rocket::futures::executor::block_on(#new_name(#test_args));
            });

            db.drop(None).await.unwrap();

            // Panic with the original error
            if let Err(cause) = result {
                std::panic::panic_any(cause);
            }
        }
    }
    .into()
}

/// Ensure signature conforms to `async fn test_ident(client_ident: Client, db_ident: Database)`.
fn check_sig(sig: Signature) -> Result<Vec<TokenStream2>, syn::Error> {
    if sig.asyncness.is_none() {
        return Err(syn::Error::new(sig.span(), "Test must be marked `async`"));
    }

    let inputs = sig.inputs;
    if inputs.len() > 2 {
        return Err(syn::Error::new(
            inputs.span(),
            "Test arguments must be a `rocket::local::asynchronous::Client` and/or a `mongodb::Database`",
        ));
    }

    let mut has_client = false;
    let mut has_db = false;
    let mut args_used = vec![];

    for input in inputs.iter() {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(_) = *pat_type.pat {
                if let Type::Path(type_path) = &*pat_type.ty {
                    if let Some(type_ident) = type_path.path.get_ident() {
                        let raw_type_ident = type_ident.to_string();
                        match raw_type_ident.as_str() {
                            "Client" => {
                                if has_client {
                                    return Err(syn::Error::new(input.span(), "Test cannot accept more than one `rocket::local::asynchronous::Client`"));
                                }
                                has_client = true;
                                args_used.push(quote! { rocket_client });
                                continue;
                            }
                            "Database" => {
                                if has_db {
                                    return Err(syn::Error::new(
                                        input.span(),
                                        "Test cannot accept more than one `mongodb::Database`",
                                    ));
                                }
                                has_db = true;
                                args_used.push(quote! { db });
                                continue;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        return Err(syn::Error::new(
            input.span(),
            "Expected one of `client_ident: Client` or `db_ident: Database`",
        ));
    }

    Ok(args_used)
}
