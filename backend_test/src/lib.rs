use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, spanned::Spanned, FnArg, GenericArgument, Ident, ItemFn, Pat, PathArguments,
    Signature, Type,
};

/// Instruments a test as a [`rocket::async_test`], injects necessary dependencies and ensures that the [`mongodb::Database`] is
/// cleared WHETHER OR NOT the test completes by passing, failing or otherwise panicking.
///
/// If a panic occurs via a failed assertion or other unwinding panic, the [`mongodb::Database`] is
/// cleared, and the panic is "rethrown".
///
/// Injectable dependencies so far include [`rocket::local::asynchronous::Client`],
/// [`mongodb::Database`] and [`crate::model::mongodb::Coll<T>`]
#[proc_macro_attribute]
pub fn backend_test(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_fn = parse_macro_input!(input as ItemFn);
    let sig = &mut item_fn.sig;

    // Extract type information and reject invalid function signatures
    let (test_args, collection_idents, collection_types) = match check_sig(sig.clone()) {
        Ok(args) => args,
        Err(err) => {
            return err.into_compile_error().into();
        }
    };

    // Rename the future so the test can have its original name
    let name = sig.ident.clone();
    let new_name = format_ident!("{}_fut", name);
    sig.ident = new_name.clone();

    // Log in the client as admin/voter if needed
    let maybe_login_as_user = parse_macro_input!(args as Option<Ident>)
        .and_then(|arg| {
            if arg == "admin" {
                Some(quote! {
                    crate::model::mongodb::Coll::<crate::model::admin::NewAdmin>::from_db(&db)
                        .insert_one(crate::model::admin::NewAdmin::example(), None)
                        .await
                        .unwrap();

                    rocket_client
                        .post(uri!(crate::api::auth::authenticate))
                        .header(rocket::http::ContentType::JSON)
                        .body(rocket::serde::json::json!(crate::model::admin::AdminCredentials::example()).to_string())
                        .dispatch()
                        .await;
                })
            } else if arg == "voter" {
                Some(quote! {
                    use crate::model::sms::Sms;

                    rocket_client
                        .get(uri!(crate::api::auth::challenge(Sms::example())))
                        .dispatch()
                        .await;

                    let cookies = rocket_client.cookies();
                    let cookie = cookies.get_private(crate::model::otp::challenge::CHALLENGE_COOKIE).unwrap();
                    let config = rocket_client.rocket().state::<crate::Config>().unwrap();
                    let challenge = crate::model::otp::challenge::Challenge::from_cookie(&cookie, config).unwrap();
                    let code = challenge.code();

                    rocket_client
                        .post(uri!(crate::api::auth::verify))
                        .header(rocket::http::ContentType::JSON)
                        .body(rocket::serde::json::json!(code).to_string())
                        .dispatch()
                        .await;
                })
            } else {
                None
            }
        })
        .unwrap_or_default();

    let stmts = item_fn.block.stmts;

    quote! {
        #[rocket::async_test]
        async fn #name() {
            let db_client = crate::db_client().await;
            let rocket_client = rocket::local::asynchronous::Client::tracked(crate::rocket_for_db_client(db_client.clone()))
                .await
                .unwrap();
            let db = db_client.database(crate::DATABASE);

            #maybe_login_as_user

            #sig {
                #(#stmts)*
            }

            // `Mutex<T>: UnwindSafe` circumvents `T: !UnwindSafe`:
            // - See https://stackoverflow.com/a/66529014/13112498
            let client_mutex = std::sync::Mutex::new(rocket_client);
            let db_mutex = std::sync::Mutex::new(db.clone());

            let result = std::panic::catch_unwind(|| {
                // Transfer mutexes across the unwind boundary
                // Unwraps are valid as no poisoning occurs: mutexes are not given to other threads
                let rocket_client = client_mutex.into_inner().unwrap();
                let db = db_mutex.into_inner().unwrap();

                #(
                    let #collection_idents = crate::model::mongodb::Coll::<#collection_types>::from_db(&db);
                )*

                // Manually run future inside the current runtime
                let handle = rocket::tokio::runtime::Handle::current();
                let _ = handle.enter();
                rocket::futures::executor::block_on(#new_name(#(#test_args),* #(,#collection_idents)*));
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
fn check_sig(sig: Signature) -> Result<(Vec<TokenStream2>, Vec<Ident>, Vec<Ident>), syn::Error> {
    if sig.asyncness.is_none() {
        return Err(syn::Error::new(sig.span(), "Test must be marked `async`"));
    }

    let mut has_client = false;
    let mut has_db = false;
    let mut args = vec![];
    let mut collection_idents = vec![];
    let mut collection_types = vec![];

    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                if let Type::Path(type_path) = &*pat_type.ty {
                    if let Some(type_ident) = type_path.path.get_ident() {
                        if type_ident == "Client" {
                            if has_client {
                                return Err(syn::Error::new(input.span(), "Test cannot accept more than one `rocket::local::asynchronous::Client`"));
                            }
                            has_client = true;
                            args.push(quote! { rocket_client });
                            continue;
                        } else if type_ident == "Database" {
                            if has_db {
                                return Err(syn::Error::new(
                                    input.span(),
                                    "Test cannot accept more than one `mongodb::Database`",
                                ));
                            }
                            has_db = true;
                            args.push(quote! { db });
                            continue;
                        }
                    } else {
                        // Valid as the last path segment for any type is itself
                        let possible_collection = type_path.path.segments.last().unwrap();
                        if possible_collection.ident == "Coll" {
                            if let PathArguments::AngleBracketed(generics) =
                                &possible_collection.arguments
                            {
                                if let Some(arg) = generics.args.first() {
                                    if let GenericArgument::Type(ty) = arg {
                                        if let Type::Path(ty_path) = ty {
                                            if let Some(ty_ident) = ty_path.path.get_ident() {
                                                collection_idents.push(pat_ident.ident.clone());
                                                collection_types.push(ty_ident.clone());
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        return Err(syn::Error::new(
            input.span(),
            "Expected one of `client_ident: Client`, `db_ident: Database` or `collection_ident: Coll<T>`",
        ));
    }

    Ok((args, collection_idents, collection_types))
}
