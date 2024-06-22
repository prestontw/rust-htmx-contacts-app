use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::Router;
use axum_extra::extract::Form;
use axum_extra::routing::RouterExt;
use axum_extra::routing::TypedPath;
use axum_flash::Flash;
use axum_flash::IncomingFlashes;
use maud::html;
use maud::Markup;
use maud::DOCTYPE;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use uuid::Uuid;

// TODO:
// - use diesel
// - style with tailwind
//   - https://www.crocodile.dev/blog/css-transitions-with-tailwind-and-htmx
//   - https://tailwindcss.com/docs/plugins#adding-variants

#[derive(Clone)]
struct AppState {
    contacts: Arc<RwLock<Vec<Contact<ContactId>>>>,
    flash_config: axum_flash::Config,
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
                first_name: "Joe1".into(),
                last_name: "Smith".into(),
                email_address: "joe.smith@example.com".into(),
                phone: "222-999-8899".into(),
                id: ContactId::new(),
            },
            Contact {
                first_name: "Joe2".into(),
                last_name: "Smith".into(),
                email_address: "joe.smith@example.com".into(),
                phone: "222-999-8899".into(),
                id: ContactId::new(),
            },
            Contact {
                first_name: "Joe3".into(),
                last_name: "Smith".into(),
                email_address: "joe.smith@example.com".into(),
                phone: "222-999-8899".into(),
                id: ContactId::new(),
            },
            Contact {
                first_name: "Joe4".into(),
                last_name: "Smith".into(),
                email_address: "joe.smith@example.com".into(),
                phone: "222-999-8899".into(),
                id: ContactId::new(),
            },
            Contact {
                first_name: "Joe5".into(),
                last_name: "Smith".into(),
                email_address: "joe.smith@example.com".into(),
                phone: "222-999-8899".into(),
                id: ContactId::new(),
            },
            Contact {
                first_name: "Joe6".into(),
                last_name: "Smith".into(),
                email_address: "joe.smith@example.com".into(),
                phone: "222-999-8899".into(),
                id: ContactId::new(),
            },
            Contact {
                first_name: "Joe7".into(),
                last_name: "Smith".into(),
                email_address: "joe.smith@example.com".into(),
                phone: "222-999-8899".into(),
                id: ContactId::new(),
            },
            Contact {
                first_name: "Joe8".into(),
                last_name: "Smith".into(),
                email_address: "joe.smith@example.com".into(),
                phone: "222-999-8899".into(),
                id: ContactId::new(),
            },
            Contact {
                first_name: "Joe9".into(),
                last_name: "Smith".into(),
                email_address: "joe.smith@example.com".into(),
                phone: "222-999-8899".into(),
                id: ContactId::new(),
            },
            Contact {
                first_name: "Joe10".into(),
                last_name: "Smith".into(),
                email_address: "joe.smith@example.com".into(),
                phone: "222-999-8899".into(),
                id: ContactId::new(),
            },
        ])),
        flash_config: axum_flash::Config::new(axum_flash::Key::generate()),
    };
    let app = Router::new()
        .typed_get(root)
        .typed_get(contacts)
        .typed_get(contacts_new_get)
        .typed_get(contacts_view)
        .typed_get(contacts_count)
        .typed_get(contacts_edit_get)
        .typed_get(contacts_email_get)
        .typed_post(contacts_new_post)
        .typed_post(contacts_edit_post)
        .typed_delete(contacts_delete)
        .typed_delete(contacts_delete_all)
        .with_state(starting_state)
        .nest_service("/dist", ServeDir::new("dist"));

    #[cfg(debug_assertions)]
    use axum::extract::Request;
    #[cfg(debug_assertions)]
    fn not_htmx_predicate<T>(req: &Request<T>) -> bool {
        !req.headers().contains_key("hx-request")
    }

    #[cfg(debug_assertions)]
    let app =
        app.layer(tower_livereload::LiveReloadLayer::new().request_predicate(not_htmx_predicate));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

impl axum::extract::FromRef<AppState> for axum_flash::Config {
    fn from_ref(state: &AppState) -> axum_flash::Config {
        state.flash_config.clone()
    }
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

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
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

/// Pending contact that is the information entered by the user. Could be
/// missing fields or have invalid fields (eg, bogus email address format).
/// Could experiment with just using a HashMap for the next endpoint.
#[derive(Deserialize, Default, Debug)]
struct PendingContact {
    #[serde(deserialize_with = "non_empty_str")]
    first_name: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    last_name: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    phone: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    email_address: Option<String>,
}

fn non_empty_str<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    use serde::Deserialize;
    let o: Option<String> = Option::deserialize(d)?;
    Ok(o.filter(|s| !s.is_empty()))
}

impl From<Contact<ContactId>> for PendingContact {
    fn from(value: Contact<ContactId>) -> Self {
        Self {
            first_name: Some(value.first_name),
            last_name: Some(value.last_name),
            phone: Some(value.phone),
            email_address: Some(value.email_address),
        }
    }
}

impl PendingContact {
    fn to_valid(
        &self,
        id: Option<ContactId>,
    ) -> Result<Contact<ContactId>, HashMap<&'static str, String>> {
        match (
            &self.first_name,
            &self.last_name,
            &self.phone,
            &self.email_address,
        ) {
            (Some(first_name), Some(last_name), Some(phone), Some(email)) if !email.is_empty() => {
                Ok(Contact {
                    id: id.unwrap_or_else(ContactId::new),
                    first_name: first_name.to_owned(),
                    last_name: last_name.to_owned(),
                    phone: phone.to_owned(),
                    email_address: email.to_owned(),
                })
            }
            _ => {
                let mut errors = HashMap::new();

                if self.first_name.as_ref() == None {
                    errors.insert("first", "Missing first name".into());
                }
                if self.last_name.as_ref() == None {
                    errors.insert("last", "Missing last name".into());
                }
                if self.phone.as_ref() == None {
                    errors.insert("phone", "Missing phone".into());
                }
                if self.email_address.as_ref().is_none()
                    || self.email_address.as_ref().is_some_and(String::is_empty)
                {
                    errors.insert("email", "Missing email address".into());
                }

                Err(errors)
            }
        }
    }
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/")]
struct Root;
async fn root(_: Root) -> impl IntoResponse {
    Redirect::permanent(&Contacts.to_string())
}

fn page(body: Markup, flashes: IncomingFlashes) -> (IncomingFlashes, Markup) {
    (
        flashes.clone(),
        html! {
            (DOCTYPE)
            head {
                script src="https://unpkg.com/htmx.org@1.9.5" crossorigin="anonymous" {}
                script defer src="https://unpkg.com/alpinejs" crossorigin="anonymous" {}
                link rel="stylesheet" href="/dist/output.css";
                script src="/dist/rsjs.js" {}
                meta charset="utf-8";
            }
            body .p-10.max-w-prose.m-auto hx-boost="true" {
                (body)

                @for flash in &flashes {
                    div .flash { (flash.1)}
                }
            }
        },
    )
}

#[derive(Debug, Deserialize)]
struct GetContactsParams {
    #[serde(rename = "q")]
    query: Option<String>,
    page: Option<u32>,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts")]
struct Contacts;

async fn contacts(
    _: Contacts,
    Query(query): Query<GetContactsParams>,
    State(state): State<AppState>,
    headers: HeaderMap,
    flashes: IncomingFlashes,
) -> impl IntoResponse {
    let page_number = query.page.unwrap_or(0);
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
            contacts
                .chunks(10)
                .skip(page_number as usize)
                .next()
                .unwrap_or_default()
                .to_vec()
        }
    };
    let contacts_len = contacts.len();
    let rows = html! {
        @for contact in contacts {
            tr {
                td {
                    input type="checkbox" name="selected_contact_ids" value=(contact.id) x-model="selected" {}
                }
                td { (contact.first_name)}
                td { (contact.last_name)}
                td { (contact.phone)}
                td { (contact.email_address)}
                td {
                    div data-overflow-menu {
                        button type="button" aria-haspopup="menu" aria-controls=(format!("contact-menu-{}", contact.id)) {"Options"}
                        div role="menu" hidden id=(format!("contact-menu-{}", contact.id)) {
                            a role="menuitem" href=(UpdateContact { id: contact.id}.to_string()) { "Edit" }
                            " "
                            a role="menuitem" href=(ViewContact { id: contact.id}.to_string()) { "View" }
                            " "
                            a role="menuitem" href="#" hx-delete=(ViewContact {id: contact.id}.to_string())
                                hx-swap="outerHTML swap:1s"
                                hx-confirm="Are you sure you want to delete this contact?"
                                hx-target="closest tr" { "Delete" }
                        }
                    }
                }
            }
        }
    };
    if headers.get("HX-Trigger").is_some_and(|val| val == "search") {
        return rows.into_response();
    }
    // todo: investigate adding new tbody when reach end of hte list
    page(
        html! {
            form .tool-bar action=(Contacts.to_string()) method="get" {
                label for="search" { "Search Term" }
                input id="search" type="search" name="q" value=(query.query.as_deref().unwrap_or_default())
                    hx-get=(Contacts.to_string())
                    hx-trigger="change, keyup delay:200ms changed"
                    hx-target="tbody"
                    hx-push-url="true"
                    hx-indicator="#spinner";
                img #spinner .htmx-indicator src="/dist/img/spinning-circles.svg" alt="Request In Flight";
                input type="submit" value="Search";
            }
            form x-data="{ selected: [] }" {
                template x-if="selected.length > 0" {
                    div .box.info.tool-bar {
                        slot x-text="selected.length" {} " contacts selected "
                        button type="button" .bad.bg.color.border
                            x-on:click=(format!("confirm(`Delete ${{selected.length}} contacts?`) && htmx.ajax('DELETE', '{}', {{ source: $root, target: document.body }})", Contacts)) { "Delete" }
                        hr aria-orientation="vertical";
                        button type="button" x-on:click="selected = []" { "Cancel" }
                    }
                }
                table {
                    thead {
                        tr {
                            th {} th {"First"} th {"Last"} th {"Phone"} th {"Email"}
                        }
                    }
                    tbody {
                        (rows)
                        @if contacts_len >= 10 {
                            tr {
                                td colspan="5" style="text-align: center" {
                                    span hx-target="closest tr"
                                        hx-trigger="revealed"
                                        hx-swap="outerHTML"
                                        hx-select="tbody > tr"
                                        hx-get=(Contacts.with_query_params(Pagination{page: page_number + 1})) { "Loading More..." }
                                }
                            }
                        }
                    }
                }
            }
            p {
                a href=(AddContact.to_string()) { "Add Contact" }
                " "
                span hx-get=(ContactsCount.to_string()) hx-trigger="revealed" {
                    img #spinner .htmx-indicator src="/dist/img/spinning-circles.svg";
                }
            }
        },
        flashes,
    ).into_response()
}

#[derive(Serialize)]
struct Pagination {
    page: u32,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/count")]
struct ContactsCount;

async fn contacts_count(_: ContactsCount, State(state): State<AppState>) -> impl IntoResponse {
    let count = state.contacts.read().await.len();
    format!("({count} total Contacts)")
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/new")]
struct AddContact;

async fn contacts_new_get(_: AddContact, flashes: IncomingFlashes) -> impl IntoResponse {
    new_contact_form(PendingContact::default(), HashMap::new(), flashes)
}

async fn contacts_new_post(
    _: AddContact,
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    flash: Flash,
    Form(pending_contact): Form<PendingContact>,
) -> impl IntoResponse {
    let contact = pending_contact.to_valid(None);
    if let Err(errors) = contact {
        return new_contact_form(pending_contact, errors, flashes).into_response();
    } else if let Ok(contact) = contact {
        let mut contacts = state.contacts.write().await;
        contacts.push(contact);
    }
    (
        flash.success("Created a new contact!"),
        Redirect::to(&Contacts.to_string()),
    )
        .into_response()
}

fn new_contact_form<'a>(
    contact: PendingContact,
    errors: HashMap<&str, String>,
    flashes: IncomingFlashes,
) -> impl IntoResponse {
    page(
        html! {
            form action=(AddContact.to_string()) method="post" {
                fieldset {
                    legend { "Contact Values" }
                    p {
                        label for="email" {"Email"}
                        input name="email_address" id="email" type="email" placeholder="Email" value=(contact.email_address.unwrap_or_default());
                        span .error {(errors.get("email").map(String::as_str).unwrap_or_default())}
                    }
                    p {
                        label for="first_name" {"First Name"}
                        input name="first_name" id="first_name" type="text" placeholder="First Name" value=(contact.first_name.unwrap_or_default());
                        span .error {(errors.get("first").map(String::as_str).unwrap_or_default())}
                    }
                    p {
                        label for="last_name" {"Last Name"}
                        input name="last_name" id="last_name" type="text" placeholder="Last Name" value=(contact.last_name.unwrap_or_default());
                        span .error {(errors.get("last").map(String::as_str).unwrap_or_default())}
                    }
                    p {
                        label for="phone" {"Phone"}
                        input name="phone" id="phone" type="text" placeholder="Phone" value=(contact.phone.unwrap_or_default());
                        span .error {(errors.get("phone").map(String::as_str).unwrap_or_default())}
                    }
                    button {"Save"}
                }
            }
            p {
                a href=(Contacts.to_string()) {"Back"}
            }
        },
        flashes,
    )
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/:id")]
struct ViewContact {
    id: ContactId,
}

async fn contacts_view(
    contact_id: ViewContact,
    State(state): State<AppState>,
    flash: Flash,
    flashes: IncomingFlashes,
) -> impl IntoResponse {
    let contact = {
        let contacts = state.contacts.read().await;
        contacts
            .iter()
            .find(|contact| contact.id == contact_id.id)
            .cloned()
    };
    if let Some(contact) = contact {
        page(
            html! {
                h1 {
                    (contact.first_name) " "  (contact.last_name)
                }
                div {
                    div { "Phone: " (contact.phone)}
                    div { "Email: " (contact.email_address)}
                }
                p {
                    a href=((UpdateContact {id: contact_id.id}).to_string()) { "Edit"}
                    " "
                    a href=(Contacts.to_string()) { "Back" }
                }
            },
            flashes,
        )
        .into_response()
    } else {
        (
            flash.warning("Could not find contact"),
            Redirect::to(&Contacts.to_string()),
        )
            .into_response()
    }
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/:id/edit")]
struct UpdateContact {
    id: ContactId,
}

async fn contacts_edit_get(
    UpdateContact { id }: UpdateContact,
    State(state): State<AppState>,
    flash: Flash,
    flashes: IncomingFlashes,
) -> impl IntoResponse {
    let contact = {
        let contacts = state.contacts.read().await;
        contacts.iter().find(|contact| contact.id == id).cloned()
    };
    if contact.is_none() {
        return (
            flash.warning("Could not find contact"),
            Redirect::to(&Contacts.to_string()),
        )
            .into_response();
    }
    edit_contact_form(contact.unwrap().into(), id, HashMap::new(), flashes).into_response()
}

async fn contacts_edit_post(
    UpdateContact { id }: UpdateContact,
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    flash: Flash,
    Form(pending_contact): Form<PendingContact>,
) -> impl IntoResponse {
    let contact = pending_contact.to_valid(Some(id));
    if let Err(errors) = contact {
        return edit_contact_form(pending_contact, id, errors, flashes).into_response();
    } else if let Ok(contact) = contact {
        let mut contacts = state.contacts.write().await;
        let found_contact = contacts.iter_mut().find(|contact| contact.id == id);
        if found_contact.is_none() {
            return (
                flash.success("Could not update not-found contact!"),
                Redirect::to(&Contacts.to_string()),
            )
                .into_response();
        }
        let found_contact = found_contact.unwrap();
        *found_contact = contact;
    }
    (
        flash.success("Updated contact!"),
        Redirect::to(&ViewContact { id }.to_string()),
    )
        .into_response()
}

fn edit_contact_form<'a>(
    contact: PendingContact,
    id: ContactId,
    errors: HashMap<&str, String>,
    flashes: IncomingFlashes,
) -> impl IntoResponse {
    page(
        html! {
            form action=(UpdateContact{id}.to_string()) method="post" {
                fieldset {
                    legend { "Contact Values" }
                    p {
                        label for="email" {"Email"}
                        input name="email_address" id="email" type="email"
                        hx-get=(ContactEmail{id: id}.to_string())
                        hx-target="next .error"
                        hx-trigger="change, keyup delay:200ms changed"
                        placeholder="Email" value=(contact.email_address.unwrap_or_default());
                        span .error {(errors.get("email").map(String::as_str).unwrap_or_default())}
                    }
                    p {
                        label for="first_name" {"First Name"}
                        input name="first_name" id="first_name" type="text" placeholder="First Name" value=(contact.first_name.unwrap_or_default());
                        span .error {(errors.get("first").map(String::as_str).unwrap_or_default())}
                    }
                    p {
                        label for="last_name" {"Last Name"}
                        input name="last_name" id="last_name" type="text" placeholder="Last Name" value=(contact.last_name.unwrap_or_default());
                        span .error {(errors.get("last").map(String::as_str).unwrap_or_default())}
                    }
                    p {
                        label for="phone" {"Phone"}
                        input name="phone" id="phone" type="text" placeholder="Phone" value=(contact.phone.unwrap_or_default());
                        span .error {(errors.get("phone").map(String::as_str).unwrap_or_default())}
                    }
                    button {"Save"}
                }
            }
            button #delete-btn hx-delete=(ViewContact{id})
                hx-target="body"
                hx-push-url="true"
                hx-confirm="Are you sure you want to delete this contact?" {"Delete Contact"}
            p {
                a href=(Contacts.to_string()) {"Back"}
            }
        },
        flashes,
    )
}

async fn contacts_delete(
    ViewContact { id }: ViewContact,
    State(state): State<AppState>,
    flash: Flash,
    headers: HeaderMap,
) -> impl IntoResponse {
    let mut contacts = state.contacts.write().await;
    let found_contact = contacts.iter().position(|contact| contact.id == id);
    if found_contact.is_none() {
        return (
            flash.success("Could not delete not-found contact!"),
            Redirect::to(&Contacts.to_string()),
        )
            .into_response();
    }
    contacts.swap_remove(found_contact.unwrap());

    if headers
        .get("HX-Trigger")
        .is_some_and(|val| val == "delete-btn")
    {
        (
            flash.success("Deleted contact!"),
            Redirect::to(&Contacts.to_string()),
        )
            .into_response()
    } else {
        return "".into_response();
    }
}

#[derive(Deserialize)]
struct DeleteContactList {
    #[serde(default)]
    selected_contact_ids: Vec<ContactId>,
}

// This is already at the `Contacts` page,
// so we don't have to redirect,
// but unsure if this is what we want.
// We might want to copy things over, but
// what if we were searching or navigating through the pages?
// Would we copy all of that logic over here?
// The example in the book renders all contacts.
async fn contacts_delete_all(
    _: Contacts,
    State(state): State<AppState>,
    flash: Flash,
    Form(to_delete): Form<DeleteContactList>,
) -> impl IntoResponse {
    let mut contacts = state.contacts.write().await;
    let mut find_error = false;
    to_delete.selected_contact_ids.into_iter().for_each(|id| {
        let found_contact = contacts.iter().position(|contact| contact.id == id);
        if found_contact.is_none() {
            find_error = true;
            return;
        }
        contacts.swap_remove(found_contact.unwrap());
    });

    if find_error {
        return (
            flash.error("Could not find one or more contacts!"),
            Redirect::to(&Contacts.to_string()),
        )
            .into_response();
    }

    (
        flash.success("Deleted contacts!"),
        Redirect::to(&Contacts.to_string()),
    )
        .into_response()
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/:id/email")]
struct ContactEmail {
    id: ContactId,
}

#[derive(Debug, Deserialize)]
struct EmailValidationParams {
    email_address: Option<String>,
}

async fn contacts_email_get(
    ContactEmail { id }: ContactEmail,
    Query(query): Query<EmailValidationParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let contact = {
        let contacts = state.contacts.read().await;
        contacts.iter().find(|contact| contact.id == id).cloned()
    };
    if contact.is_none() {
        return "".into_response();
    }
    let mut contact: PendingContact = contact.unwrap().into();
    contact.email_address = query.email_address;
    contact
        .to_valid(Some(id))
        .err()
        .and_then(|errors| errors.get("email").cloned())
        .unwrap_or_default()
        .into_response()
}
