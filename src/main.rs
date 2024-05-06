use axum::extract::Query;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::Router;
use axum_extra::routing::RouterExt;
use axum_extra::routing::TypedPath;
use maud::html;
use maud::Markup;
use maud::DOCTYPE;
use serde::Deserialize;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .typed_get(root)
        .typed_get(contacts)
        .nest_service("/dist", ServeDir::new("dist"));

    #[cfg(debug_assertions)]
    let app = app.layer(tower_livereload::LiveReloadLayer::new());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

struct Contact {
    first_name: String,
    last_name: String,
    phone: String,
    email_address: String,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/")]
struct Root;
async fn root(_: Root) -> impl IntoResponse {
    Redirect::permanent(&Contacts.to_string())
}

fn page(body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        head {
            script src="https://unpkg.com/htmx.org@1.9.5" {}
            meta charset="utf-8";
        }
        body .p-10.max-w-prose.m-auto {
            (body)
        }
    }
}

#[derive(Debug, Deserialize)]
struct GetContactsParams {
    #[serde(rename = "q")]
    query: Option<String>,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts")]
struct Contacts;
async fn contacts(_: Contacts, Query(query): Query<GetContactsParams>) -> impl IntoResponse {
    let content = if let Some(q) = query.query {
        q
    } else {
        "Hello world yes".to_string()
    };
    return page(html! { p {(content)}});
}
