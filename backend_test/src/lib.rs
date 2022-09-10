use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, spanned::Spanned, FnArg, GenericArgument, Ident, ItemFn, Pat, PathArguments,
    Signature, Type,
};

/// Transform an asynchronous test into a synchronous one, inject dependencies,
/// and ensure that the database is cleared regardless of how the test terminates.
///
/// Injectable dependencies are [`rocket::local::asynchronous::Client`],
/// [`mongodb::Database`], and [`crate::model::mongodb::Coll<T>`].
#[proc_macro_attribute]
pub fn backend_test(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_fn = parse_macro_input!(input as ItemFn);

    // Extract type information and reject invalid function signatures.
    let (test_args, collection_idents, collection_types) = match check_sig(item_fn.sig.clone()) {
        Ok(args) => args,
        Err(err) => {
            return err.into_compile_error().into();
        }
    };

    // Rename the future so the test can have its original name.
    let name = item_fn.sig.ident.clone();
    let new_name = format_ident!("{}_fut", name);
    item_fn.sig.ident = new_name.clone();

    // Log in the client as admin/voter if needed.
    let maybe_login = parse_macro_input!(args as Option<Ident>)
        .and_then(|arg| {
            if arg == "admin" {
                Some(quote! {
                    crate::model::mongodb::Coll::<crate::model::db::admin::NewAdmin>::from_db(&db)
                        .insert_one(crate::model::db::admin::NewAdmin::example(), None)
                        .await
                        .unwrap();

                    rocket_client
                        .post(uri!(crate::api::auth::authenticate))
                        .header(rocket::http::ContentType::JSON)
                        .body(rocket::serde::json::json!(crate::model::api::admin::AdminCredentials::example1()).to_string())
                        .dispatch()
                        .await;
                })
            } else if arg == "voter" {
                Some(quote! {
                    use crate::model::api::sms::Sms;

                    rocket_client
                        .post(uri!(crate::api::auth::challenge))
                        .header(rocket::http::ContentType::JSON)
                        .body(rocket::serde::json::json!(crate::model::api::auth::VoterChallengeRequest::example()).to_string())
                        .dispatch()
                        .await;

                    let cookies = rocket_client.cookies();
                    let cookie = cookies.get_private(crate::model::api::otp::CHALLENGE_COOKIE).unwrap();
                    let config = rocket_client.rocket().state::<crate::Config>().unwrap();
                    let challenge = crate::model::api::otp::Challenge::from_cookie(&cookie, config).unwrap();
                    let challenge_response = crate::model::api::auth::VoterVerifyRequest::example(challenge.code);

                    rocket_client
                        .post(uri!(crate::api::auth::verify))
                        .header(rocket::http::ContentType::JSON)
                        .body(rocket::serde::json::json!(challenge_response).to_string())
                        .dispatch()
                        .await;
                })
            } else {
                None
            }
        })
        .unwrap_or_default();

    // Rewrite the test function.
    quote! {
        #[test]
        fn #name() {
            /// Test setup.
            async fn setup() -> (rocket::local::asynchronous::Client, mongodb::Database) {
                let db_client = crate::db_client().await;
                let db_name = crate::database();
                let notifier = aws_sdk_sns::Client::new(&aws_config::load_from_env().await);
                let rocket_client = rocket::local::asynchronous::Client::tracked(crate::rocket_for_db_and_notifier(db_client.clone(), &db_name, notifier).await)
                    .await
                    .unwrap();
                let db = db_client.database(&db_name);

                #maybe_login

                (rocket_client, db)
            }

            /// The test itself.
            #item_fn

            /// Test cleanup.
            async fn cleanup(db: mongodb::Database) {
                db.drop(None).await.unwrap();
            }

            // Create an async runtime. We need a separate one for inside and
            // outside the `catch_unwind`.
            let outer_runtime = rocket::tokio::runtime::Builder::new_multi_thread()
                .thread_name("test-setup-cleanup")
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap();
            let inner_runtime = rocket::tokio::runtime::Builder::new_multi_thread()
                .thread_name("rocket-worker-test-thread")
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap();

            // Run the setup.
            let (rocket_client, db) = outer_runtime.block_on(setup());

            // Run the test, catching any panics.
            // Use mutexes to safely transfer `!UnwindSafe` data.
            let client_mutex = std::sync::Mutex::new(rocket_client);
            let db_mutex = std::sync::Mutex::new(db.clone());
            let runtime_mutex = std::sync::Mutex::new(inner_runtime);
            let result = std::panic::catch_unwind(|| {
                let rocket_client = client_mutex.into_inner().unwrap();
                let db = db_mutex.into_inner().unwrap();
                let runtime = runtime_mutex.into_inner().unwrap();

                #(
                    let #collection_idents = crate::model::mongodb::Coll::<#collection_types>::from_db(&db);
                )*

                runtime.block_on(#new_name(#(#test_args),* #(,#collection_idents)*));
            });

            // Run the cleanup.
            outer_runtime.block_on(cleanup(db));

            // If the test panicked, re-raise the panic.
            if let Err(cause) = result {
                std::panic::panic_any(cause);
            }
        }
    }
    .into()
}

/// Ensure the wrapped test is async, extract parameters to inject, and reject unknown parameters.
#[allow(clippy::type_complexity)]
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
                                if let Some(GenericArgument::Type(Type::Path(type_path))) =
                                    generics.args.first()
                                {
                                    if let Some(type_ident) = type_path.path.get_ident() {
                                        collection_idents.push(pat_ident.ident.clone());
                                        collection_types.push(type_ident.clone());
                                        continue;
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
