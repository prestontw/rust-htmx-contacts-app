#![feature(type_changing_struct_update)]

use std::collections::HashMap;
use std::env;
use std::fmt::Display;

use axum::body::Body;
use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::response::Response;
use axum::Router;
use axum_extra::extract::Form;
use axum_extra::routing::RouterExt;
use axum_extra::routing::TypedPath;
use axum_flash::Flash;
use axum_flash::IncomingFlashes;
use deadpool_diesel::postgres::Manager;
use deadpool_diesel::postgres::Pool;
use deadpool_diesel::Runtime;
use diesel::prelude::*;
use diesel::query_builder::AsChangeset;
use diesel::sql_types::Integer;
use diesel::Queryable;
use diesel::RunQueryDsl;
use diesel::Selectable;
use diesel::SelectableHelper;
use diesel_derive_newtype::DieselNewType;
use dotenvy::dotenv;
use maud::html;
use maud::Markup;
use maud::DOCTYPE;
use serde::Deserialize;
use serde::Serialize;
use tower_http::services::ServeDir;

// TODO:
// - use diesel
// - style with tailwind
//   - https://www.crocodile.dev/blog/css-transitions-with-tailwind-and-htmx
//   - https://tailwindcss.com/docs/plugins#adding-variants

#[derive(Clone)]
struct AppState {
    db_pool: Pool,
    flash_config: axum_flash::Config,
}

fn establish_connection() -> Pool {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = Manager::new(&database_url, Runtime::Tokio1);
    Pool::builder(manager)
        .max_size(8)
        .build()
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[tokio::main]
async fn main() {
    let pool = establish_connection();
    let starting_state = AppState {
        db_pool: pool,
        flash_config: axum_flash::Config::new(axum_flash::Key::generate()),
    };
    let api_routes = Router::new()
        .typed_get(api::get_contacts)
        .typed_get(api::get_contact)
        .typed_put(api::update_contact)
        .typed_delete(api::delete_contact)
        .typed_post(api::new_contact);

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
        .nest("/api/v1", api_routes)
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

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Pool error: {0}")]
    PoolError(#[from] deadpool_diesel::postgres::PoolError),
    #[error("PostgreSQL error: {0}")]
    DieselError(#[from] diesel::result::Error),
    #[error("Deadpool error: {0}")]
    DeadpoolError(#[from] deadpool_diesel::InteractError),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "An internal error occurred. Please try again later.",
        )
            .into_response()
    }
}

pub trait IdType<T>: Copy + std::fmt::Display {
    type Id;

    /// Returns the inner ID.
    fn id(self) -> Self::Id;
}

#[derive(Copy, Clone, Debug, Default)]
pub struct NoId;

impl<T> IdType<T> for NoId {
    type Id = std::convert::Infallible;

    fn id(self) -> Self::Id {
        unreachable!("Cannot access non-ID")
    }
}

impl<DB> diesel::serialize::ToSql<Integer, DB> for NoId
where
    DB: diesel::backend::Backend,
    i32: diesel::serialize::ToSql<Integer, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        0.to_sql(out)
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

#[derive(DieselNewType, Clone, Copy, Debug, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(transparent)]
struct ContactId(i32);

impl Display for ContactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(AsChangeset, Queryable, Selectable, Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = hypermedia_systems_rust::schema::contacts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct Contact {
    id: ContactId,
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

impl From<Contact> for PendingContact {
    fn from(value: Contact) -> Self {
        Self {
            first_name: Some(value.first_name),
            last_name: Some(value.last_name),
            phone: Some(value.phone),
            email_address: Some(value.email_address),
        }
    }
}

impl PendingContact {
    fn to_valid(&self) -> Result<Contact, HashMap<&'static str, String>> {
        match (
            &self.first_name,
            &self.last_name,
            &self.phone,
            &self.email_address,
        ) {
            (Some(first_name), Some(last_name), Some(phone), Some(email)) if !email.is_empty() => {
                Ok(Contact {
                    id: ContactId::default(),
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
                script src="//unpkg.com/hyperscript.org" crossorigin="anonymous" {}
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
    Query(GetContactsParams {
        query,
        page: page_number,
    }): Query<GetContactsParams>,
    State(state): State<AppState>,
    headers: HeaderMap,
    flashes: IncomingFlashes,
) -> Result<Response<Body>, Error> {
    let page_number = page_number.unwrap_or(0);
    let contacts = {
        use hypermedia_systems_rust::schema::contacts::dsl::*;

        let connection = state.db_pool.get().await.map_err(Error::PoolError)?;
        let search_string = query.clone();
        connection
            .interact(move |connection| {
                if let Some(q) = search_string.clone() {
                    contacts
                        .filter(
                            first_name
                                .ilike(format!("{}%", q))
                                .or(last_name.ilike(format!("{}%", q))),
                        )
                        .select(Contact::as_select())
                        .load(connection)
                        .map_err(Error::DieselError)
                } else {
                    contacts
                        .order(id)
                        .limit(10)
                        .offset(page_number.into())
                        .select(Contact::as_select())
                        .load(connection)
                        .map_err(Error::DieselError)
                }
            })
            .await
            .map_err(Error::DeadpoolError)??
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
                            a role="menuitem" href="" { "Edit" }
                            " "
                            a role="menuitem" href="" { "View" }
                            " "
                            a role="menuitem" href="#" hx-delete="Hello"
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
        return Ok(rows.into_response());
    }
    // todo: investigate adding new tbody when reach end of hte list
    Ok(page(
        html! {
            form .tool-bar action=(Contacts.to_string()) method="get" {
                label for="search" { "Search Term" }
                input id="search" type="search" name="q" placeholder="Search Contacts"
                _="on keydown[altKey and code is 'KeyS'] from the window me.focus()" value=(query.as_deref().unwrap_or_default())
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
                a href="Add Contact" { "Add Contact" }
                " "
                span hx-get=(ContactsCount.to_string()) hx-trigger="revealed" {
                    img #spinner .htmx-indicator src="/dist/img/spinning-circles.svg";
                }
            }
        },
        flashes,
    ).into_response())
}

#[derive(Serialize)]
struct Pagination {
    page: u32,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/count")]
struct ContactsCount;

async fn contacts_count(_: ContactsCount, State(state): State<AppState>) -> Result<String, Error> {
    let pool = state.db_pool.get().await.map_err(Error::PoolError)?;
    let count: i64 = pool
        .interact(|connection| {
            use hypermedia_systems_rust::schema::contacts::dsl::*;

            contacts
                .count()
                .get_result(connection)
                .map_err(Error::DieselError)
        })
        .await
        .map_err(Error::DeadpoolError)??;
    Ok(format!("({} total Contacts)", count))
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
) -> Result<Response<Body>, Error> {
    let contact = pending_contact.to_valid();
    if let Err(errors) = contact {
        return Ok(new_contact_form(pending_contact, errors, flashes).into_response());
    } else if let Ok(contact) = contact {
        use hypermedia_systems_rust::schema::contacts;

        let connection = state.db_pool.get().await.map_err(Error::PoolError)?;
        connection
            .interact(|connection| {
                diesel::insert_into(contacts::table)
                    .values(contact)
                    .returning(Contact::as_returning())
                    .execute(connection)
            })
            .await
            .map_err(Error::DeadpoolError)??;
    }
    Ok((
        flash.success("Created a new contact!"),
        Redirect::to(&Contacts.to_string()),
    )
        .into_response())
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

async fn find_contact(pool: Pool, contact_id: ContactId) -> Result<Contact, Error> {
    let connection = pool.get().await.map_err(Error::PoolError)?;
    let contact = connection
        .interact(move |connection| {
            use hypermedia_systems_rust::schema::contacts::dsl::*;

            let contact: Contact = contacts
                .find(contact_id)
                .select(Contact::as_select())
                .first(connection)
                .map_err(Error::DieselError)?;
            Ok::<Contact, Error>(contact)
        })
        .await
        .map_err(Error::DeadpoolError)??
        .clone();
    Ok(contact)
}

async fn contacts_view(
    ViewContact { id }: ViewContact,
    State(state): State<AppState>,
    flash: Flash,
    flashes: IncomingFlashes,
) -> Result<Response<Body>, Error> {
    let contact = find_contact(state.db_pool, id).await;
    if let Ok(contact) = contact {
        Ok(page(
            html! {
                h1 {
                    (contact.first_name) " "  (contact.last_name)
                }
                div {
                    div { "Phone: " (contact.phone)}
                    div { "Email: " (contact.email_address)}
                }
                p {
                    a href=((UpdateContact {id}).to_string()) { "Edit"}
                    " "
                    a href=(Contacts.to_string()) { "Back" }
                }
            },
            flashes,
        )
        .into_response())
    } else {
        Ok((
            flash.warning("Could not find contact"),
            Redirect::to(&Contacts.to_string()),
        )
            .into_response())
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
    let contact = find_contact(state.db_pool, id).await;
    if contact.is_err() {
        return (
            flash.warning("Could not find contact"),
            Redirect::to(&Contacts.to_string()),
        )
            .into_response();
    }
    edit_contact_form(id, contact.unwrap().into(), HashMap::new(), flashes).into_response()
}

async fn contacts_edit_post(
    UpdateContact { id }: UpdateContact,
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    flash: Flash,
    Form(pending_contact): Form<PendingContact>,
) -> Result<Response<Body>, Error> {
    let contact = pending_contact.to_valid();
    if let Err(errors) = contact {
        return Ok(edit_contact_form(id, pending_contact, errors, flashes).into_response());
    } else if let Ok(contact) = contact {
        let connection = state.db_pool.get().await.map_err(Error::PoolError)?;
        connection
            .interact(|connection| {
                use hypermedia_systems_rust::schema::contacts::dsl::*;

                let contact_id = id;
                diesel::update(contacts.find(contact_id))
                    .set(contact)
                    .execute(connection)
            })
            .await
            .map_err(Error::DeadpoolError)??;
    }
    Ok((
        flash.success("Updated contact!"),
        Redirect::to(&ViewContact { id }.to_string()),
    )
        .into_response())
}

fn edit_contact_form<'a>(
    id: ContactId,
    contact: PendingContact,
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
                        hx-get=(ContactEmail{id}.to_string())
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
    ViewContact { id: contact_id }: ViewContact,
    State(state): State<AppState>,
    flash: Flash,
    headers: HeaderMap,
) -> Result<Response<Body>, Error> {
    let connection = state.db_pool.get().await.map_err(Error::PoolError)?;
    connection
        .interact(move |connection| {
            use hypermedia_systems_rust::schema::contacts::dsl::*;

            diesel::delete(contacts.find(contact_id))
                .execute(connection)
                .map_err(Error::DieselError)?;
            Ok::<(), Error>(())
        })
        .await??;

    if headers
        .get("HX-Trigger")
        .is_some_and(|val| val == "delete-btn")
    {
        Ok((
            flash.success("Deleted contact!"),
            Redirect::to(&Contacts.to_string()),
        )
            .into_response())
    } else {
        Ok("".into_response())
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
) -> Result<Response<Body>, Error> {
    let connection = state.db_pool.get().await.map_err(Error::PoolError)?;
    connection
        .interact(|connection| {
            use hypermedia_systems_rust::schema::contacts::dsl::*;

            diesel::delete(contacts.filter(id.eq_any(to_delete.selected_contact_ids)))
                .execute(connection)
                .map_err(Error::DieselError)?;
            Ok::<(), Error>(())
        })
        .await
        .map_err(Error::DeadpoolError)??;

    Ok((
        flash.success("Deleted contacts!"),
        Redirect::to(&Contacts.to_string()),
    )
        .into_response())
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
    _: ContactEmail,
    Query(query): Query<EmailValidationParams>,
    State(state): State<AppState>,
) -> Result<Response<Body>, Error> {
    let email = query.email_address.unwrap_or_default();
    if email == "" {
        return Ok("Email cannot be empty".into_response());
    }

    let connection = state.db_pool.get().await.map_err(Error::PoolError)?;
    let contact_count: i64 = connection
        .interact(|connection| {
            use hypermedia_systems_rust::schema::contacts::dsl::*;

            contacts
                .filter(email_address.like(email))
                .count()
                .get_result(connection)
        })
        .await
        .map_err(Error::DeadpoolError)??;
    if contact_count == 0 {
        Ok("".into_response())
    } else {
        Ok("Email must be unique".into_response())
    }
}

mod api {
    use axum::body::Body;
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::response::Response;
    use axum::Json;
    use diesel::prelude::*;
    use diesel::QueryDsl;
    use diesel::RunQueryDsl;
    use diesel::SelectableHelper;
    use serde::Serialize;

    use crate::AppState;
    use crate::Contact;
    use crate::Contacts;
    use crate::Error;
    use crate::ViewContact;

    pub async fn get_contacts(
        _: Contacts,
        State(state): State<AppState>,
    ) -> Result<Response<Body>, Error> {
        #[derive(Serialize)]
        struct Contacts {
            contacts: Vec<Contact>,
        }

        let connection = state.db_pool.get().await.map_err(Error::PoolError)?;
        let contacts: Vec<Contact> = connection
            .interact(|connection| {
                use hypermedia_systems_rust::schema::contacts::dsl::*;

                contacts
                    .select(Contact::as_select())
                    .get_results(connection)
            })
            .await
            .map_err(Error::DeadpoolError)??;

        Ok(Json(Contacts { contacts }).into_response())
    }

    pub async fn get_contact(
        ViewContact { id: contact_id }: ViewContact,
        State(state): State<AppState>,
    ) -> Result<Response<Body>, Error> {
        let connection = state.db_pool.get().await.map_err(Error::PoolError)?;
        let contact: Option<Contact> = connection
            .interact(move |connection| {
                use hypermedia_systems_rust::schema::contacts::dsl::*;

                contacts.find(contact_id).first(connection).optional()
            })
            .await
            .map_err(Error::DeadpoolError)??;
        match contact {
            None => Ok((StatusCode::NOT_FOUND, "Could not find contact").into_response()),
            Some(contact) => Ok(Json(contact).into_response()),
        }
    }

    pub async fn update_contact(
        ViewContact { id: contact_id }: ViewContact,
        State(state): State<AppState>,
        Json(contact): Json<Contact>,
    ) -> Result<Response<Body>, Error> {
        let connection = state.db_pool.get().await.map_err(Error::PoolError)?;
        let contact = connection
            .interact(move |connection| {
                use hypermedia_systems_rust::schema::contacts::dsl::*;

                diesel::update(contacts.find(contact_id))
                    .set(contact)
                    .returning(Contact::as_returning())
                    .get_result(connection)
            })
            .await
            .map_err(Error::DeadpoolError)??;
        Ok(Json(contact).into_response())
    }

    pub async fn delete_contact(
        ViewContact { id: contact_id }: ViewContact,
        State(state): State<AppState>,
    ) -> Result<Response<Body>, Error> {
        let connection = state.db_pool.get().await.map_err(Error::PoolError)?;
        connection
            .interact(move |connection| {
                use hypermedia_systems_rust::schema::contacts::dsl::*;

                diesel::delete(contacts.find(contact_id))
                    .execute(connection)
                    .map_err(Error::DieselError)?;
                Ok::<(), Error>(())
            })
            .await
            .map_err(Error::DeadpoolError)??;
        Ok((StatusCode::OK, "Successfully deleted").into_response())
    }

    pub async fn new_contact(
        _: Contacts,
        State(state): State<AppState>,
        Json(new_contact): Json<Contact>,
    ) -> Result<Json<Contact>, Error> {
        let connection = state.db_pool.get().await.map_err(Error::PoolError)?;
        let new_contact = connection
            .interact(|connection| {
                use hypermedia_systems_rust::schema::contacts;

                diesel::insert_into(contacts::table)
                    .values(new_contact)
                    .returning(Contact::as_returning())
                    .get_result(connection)
            })
            .await
            .map_err(Error::DeadpoolError)??;
        Ok(Json(new_contact))
    }
}
