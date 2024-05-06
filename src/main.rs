use std::sync::Arc;

use axum::extract::Query;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::Router;
use axum_extra::routing::RouterExt;
use axum_extra::routing::TypedPath;
use maud::html;
use maud::Markup;
use maud::DOCTYPE;
use serde::Deserialize;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

#[derive(Clone)]
struct AppState {
    contacts: Arc<RwLock<Vec<Contact>>>,
}

#[tokio::main]
async fn main() {
    let starting_state = AppState {
        contacts: Arc::new(RwLock::new(vec![Contact {
            first_name: "Hello".into(),
            last_name: "World".into(),
            email_address: "".into(),
            phone: "".into(),
        }])),
    };
    let app = Router::new()
        .typed_get(root)
        .typed_get(contacts)
        .with_state(starting_state)
        .nest_service("/dist", ServeDir::new("dist"));

    #[cfg(debug_assertions)]
    let app = app.layer(tower_livereload::LiveReloadLayer::new());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone, Debug)]
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
async fn contacts(
    _: Contacts,
    Query(query): Query<GetContactsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let contacts = {
        let contacts = state.contacts.read().await;
        if let Some(q) = query.query {
            contacts
                .iter()
                .filter(|contact: &&Contact| contact.first_name.contains(&q))
                .cloned()
                .collect::<Vec<Contact>>()
        } else {
            contacts.clone()
        }
    };
    return page(html! { p {(format!("{:?}", contacts))}});
}
