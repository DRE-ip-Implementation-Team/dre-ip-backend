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
/// Note: this attribute requires that [`rocket::async_test`] and `client_and_db` are
/// in scope. A dev dependency on `tokio` is required to run each test future within the closure
/// passed to [`std::panic::catch_unwind`].
pub fn db_test(_: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_fn = parse_macro_input!(input as ItemFn);

    // Reject invalid function signatures
    if let Err(err) = check_sig(item_fn.sig.clone()) {
        return err.into_compile_error().into();
    }

    // Rename the future so the test is named as expected
    let name = item_fn.sig.ident;
    let new_name = format_ident!("{}_fut", name);
    item_fn.sig.ident = new_name.clone();

    quote! {
        #[rocket::async_test]
        async fn #name() {
            let (client, db) = client_and_db().await;

            #item_fn

            // `Mutex<T>: UnwindSafe` circumvents `T: !UnwindSafe`:
            // - See https://stackoverflow.com/a/66529014/13112498
            let client_mutex = std::sync::Mutex::new(client);
            let db_mutex = std::sync::Mutex::new(db.clone());

            let result = std::panic::catch_unwind(|| {
                // Transfer mutexes across the unwind boundary
                // Unwraps are valid as no poisoning occurs: mutexes are not given no other threads
                let client = client_mutex.into_inner().unwrap();
                let db = db_mutex.into_inner().unwrap();

                // Manually run future inside
                let handle = tokio::runtime::Handle::current();
                let _ = handle.enter();
                futures::executor::block_on(#new_name(client, db));
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
fn check_sig(sig: Signature) -> Result<(), syn::Error> {
    if sig.asyncness.is_none() {
        return Err(syn::Error::new(sig.span(), "Test must be marked `async`"));
    }

    let inputs = sig.inputs;
    if inputs.len() != 2 {
        return Err(syn::Error::new(
            inputs.span(),
            "Test arguments must be a `rocket::local::asynchronous::Client` and a `mongodb::Database`",
        ));
    }

    let mut has_client = false;
    let mut has_db = false;

    for input in &inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(_) = *pat_type.pat {
                if let Type::Path(type_path) = &*pat_type.ty {
                    if let Some(type_ident) = type_path.path.get_ident() {
                        let raw_type_ident = type_ident.to_string();
                        match raw_type_ident.as_str() {
                            "Client" => {
                                has_client = true;
                                continue;
                            }
                            "Database" => {
                                has_db = true;
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

    if has_client && has_db {
        Ok(())
    } else {
        Err(syn::Error::new(
            inputs.span(),
            "Test must accept a `rocket::local::asynchronous::Client` and `mongodb::Database`",
        ))
    }
}
