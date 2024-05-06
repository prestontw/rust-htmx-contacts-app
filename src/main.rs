use axum::response::IntoResponse;
use axum::response::Redirect;
use axum_extra::routing::TypedPath;
use serde::Deserialize;

fn main() {
    println!("Hello, world!");
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/")]
struct Root;
async fn root(_: Root) -> impl IntoResponse {
    Redirect::permanent(&Contacts.to_string())
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts")]
struct Contacts;
async fn contacts(_: Contacts) -> impl IntoResponse {
    "Hello world"
}
