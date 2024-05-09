use std::fmt::Display;
use std::sync::Arc;

use axum::extract::Query;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::Form;
use axum::Router;
use axum_extra::routing::RouterExt;
use axum_extra::routing::TypedPath;
use maud::html;
use maud::Markup;
use maud::DOCTYPE;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    contacts: Arc<RwLock<Vec<Contact<ContactId>>>>,
}

#[tokio::main]
async fn main() {
    let starting_state = AppState {
        contacts: Arc::new(RwLock::new(vec![
            Contact {
                first_name: "Hello".into(),
                last_name: "World".into(),
                email_address: "".into(),
                phone: "".into(),
                id: ContactId::new(),
            },
            Contact {
                first_name: "Joe".into(),
                last_name: "Smith".into(),
                email_address: "joe.smith@example.com".into(),
                phone: "222-999-8899".into(),
                id: ContactId::new(),
            },
        ])),
    };
    let app = Router::new()
        .typed_get(root)
        .typed_get(contacts)
        .typed_get(contacts_new_get)
        .typed_post(contacts_new_post)
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

pub trait IdType<T>: Copy + std::fmt::Display {
    type Id;

    /// Returns the inner ID.
    fn id(self) -> Self::Id;
}

#[derive(Copy, Clone)]
pub struct NoId;

impl<T> IdType<T> for NoId {
    type Id = std::convert::Infallible;

    fn id(self) -> Self::Id {
        unreachable!("Cannot access non-ID")
    }
}

/// NoId can be deserialized from any source, even if the field is not
/// present.
/// From https://boinkor.net/2024/04/some-useful-types-for-database-using-rust-web-apps.
impl<'de> Deserialize<'de> for NoId {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(NoId)
    }
}

impl Display for NoId {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(transparent)]
struct ContactId(uuid::Uuid);

impl IdType<ContactId> for ContactId {
    type Id = Uuid;

    fn id(self) -> Self::Id {
        self.0
    }
}

impl Display for ContactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ContactId {
    fn new() -> Self {
        ContactId(uuid::Uuid::new_v4())
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Contact<ID: IdType<ContactId>> {
    id: ID,
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

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/:id/edit")]
struct UpdateContact {
    id: ContactId,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/:id")]
struct ViewContact {
    id: ContactId,
}

async fn contacts(
    _: Contacts,
    Query(query): Query<GetContactsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let contacts = {
        let contacts = state.contacts.read().await;
        if let Some(q) = &query.query {
            contacts
                .iter()
                .filter(|contact| {
                    contact.first_name.contains(q)
                        || contact.last_name.contains(q)
                        || contact.email_address.contains(q)
                        || contact.phone.contains(q)
                })
                .cloned()
                .collect::<Vec<Contact<_>>>()
        } else {
            contacts.clone()
        }
    };
    page(html! {
        form action=(Contacts.to_string()) method="get" {
            label for="search" { "Search Term" }
            input id="search" type="search" name="q" value=(query.query.unwrap_or_else(String::new));
            input type="submit" value="Search";
        }
        table {
            thead {
                tr {
                    th {"First"} th {"Last"} th {"Phone"} th {"Email"}
                }
            }
            tbody {
                @for contact in contacts {
                    tr {
                        td { (contact.first_name)}
                        td { (contact.last_name)}
                        td { (contact.phone)}
                        td { (contact.email_address)}
                        td {
                            a href=(UpdateContact { id: contact.id}.to_string()) { "Edit" }
                            " "
                            a href=(ViewContact { id: contact.id}.to_string()) { "View" }
                        }
                    }
                }
            }
        }
        p {
            a href=(AddContact.to_string()) { "Add Contact" }
        }
    })
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/new")]
struct AddContact;

async fn contacts_new_get(_: AddContact) -> impl IntoResponse {
    new_contact_form(None)
}

#[axum::debug_handler]
async fn contacts_new_post(_: AddContact, Form(contact): Form<Contact<NoId>>) -> impl IntoResponse {
    println!("{:?}", contact.first_name)
}

fn new_contact_form(contact: Option<&Contact<NoId>>) -> Markup {
    page(html! {
        form action=(AddContact.to_string()) method="post" {
            fieldset {
                legend { "Contact Values" }
                p {
                    label for="email" {"Email"}
                    input name="email_address" id="email" type="email" placeholder="Email" value=(contact.map(|contact| contact.email_address.clone()).unwrap_or_default());
                    span .error {}
                }
                p {
                    label for="first_name" {"First Name"}
                    input name="first_name" id="first_name" type="text" placeholder="First Name" value=(contact.map(|contact| contact.first_name.clone()).unwrap_or_default());
                    span .error {}
                }
                p {
                    label for="last_name" {"Last Name"}
                    input name="last_name" id="last_name" type="text" placeholder="Last Name" value=(contact.map(|contact| contact.last_name.clone()).unwrap_or_default());
                    span .error {}
                }
                p {
                    label for="phone" {"Phone"}
                    input name="phone" id="phone" type="text" placeholder="Phone" value=(contact.map(|contact| contact.phone.clone()).unwrap_or_default());
                    span .error {}
                }
                button {"Save"}
            }
        }
        p {
            a href=(Contacts.to_string()) {"Back"}
        }
    })
}
