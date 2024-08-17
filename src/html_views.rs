use std::collections::HashMap;

use axum::body::Body;
use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderName;
use axum::http::HeaderValue;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::response::Response;
use axum_extra::extract::Form;
use axum_extra::routing::TypedPath;
use axum_extra::TypedHeader;
use axum_flash::Flash;
use axum_flash::IncomingFlashes;
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use maud::html;
use maud::Markup;
use maud::DOCTYPE;
use serde::Deserialize;
use serde::Serialize;

use crate::model::Contact;
use crate::model::ContactId;
use crate::model::PendingContact;
use crate::AppError;
use crate::AppState;

#[derive(Deserialize, TypedPath)]
#[typed_path("/")]
pub struct Root;

pub async fn root(_: Root) -> impl IntoResponse {
    Redirect::permanent(&Contacts.to_string())
}

pub fn page(body: Markup, flashes: IncomingFlashes) -> (IncomingFlashes, Markup) {
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
pub struct GetContactsParams {
    #[serde(rename = "q")]
    pub query: Option<String>,
    pub page: Option<u32>,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts")]
pub struct Contacts;

pub async fn contacts(
    _: Contacts,
    Query(GetContactsParams {
        query,
        page: page_number,
    }): Query<GetContactsParams>,
    State(state): State<AppState>,
    contacts_action: Option<TypedHeader<ContactsInteraction>>,
    flashes: IncomingFlashes,
) -> Result<Response<Body>, AppError> {
    let page_number = page_number.unwrap_or(0);
    let contacts = {
        let connection = state.db_pool.get().await?;
        let search_string = query.clone();
        connection
            .interact(move |connection| {
                use crate::schema::contacts::dsl::contacts;
                use crate::schema::contacts::dsl::first_name;
                use crate::schema::contacts::dsl::id;
                use crate::schema::contacts::dsl::last_name;

                if let Some(q) = search_string.clone() {
                    contacts
                        .filter(
                            first_name
                                .ilike(format!("{}%", q))
                                .or(last_name.ilike(format!("{}%", q))),
                        )
                        .select(Contact::as_select())
                        .load(connection)
                } else {
                    contacts
                        .order(id)
                        .limit(10)
                        .offset(page_number.into())
                        .select(Contact::as_select())
                        .load(connection)
                }
            })
            .await??
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
                            a role="menuitem" href=(UpdateContact {id: contact.id}.to_string()) { "Edit" }
                            " "
                            a role="menuitem" href=(ViewContact {id: contact.id}.to_string()) { "View" }
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
    if matches!(
        contacts_action.as_deref(),
        Some(ContactsInteraction::Search)
    ) {
        return Ok(rows.into_response());
    }
    // todo: investigate adding new tbody when reach end of hte list
    Ok(page(
            html! {
                form .tool-bar action=(Contacts.to_string()) method="get" {
                    label for=(ContactsInteraction::Search.id()) { "Search Term" }
                    input id=(ContactsInteraction::Search.id()) type="search" name="q" placeholder="Search Contacts"
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
                    a href=(AddContact.to_string()) { "Add Contact" }
                    " "
                    span hx-get=(ContactsCount.to_string()) hx-trigger="revealed" {
                        img #spinner .htmx-indicator src="/dist/img/spinning-circles.svg";
                    }
                }
            },
            flashes,
        ).into_response())
}

pub enum ContactsInteraction {
    Search,
}

impl ContactsInteraction {
    fn id(&self) -> &'static str {
        match self {
            Self::Search => "search",
        }
    }
}

impl axum_extra::headers::Header for ContactsInteraction {
    fn name() -> &'static axum::http::HeaderName {
        &HX_TRIGGER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum_extra::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i axum::http::HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(axum_extra::headers::Error::invalid)?;

        if value == Self::Search.id() {
            Ok(Self::Search)
        } else {
            Err(axum_extra::headers::Error::invalid())
        }
    }

    fn encode<E: Extend<axum::http::HeaderValue>>(&self, values: &mut E) {
        let s = self.id();
        let value = HeaderValue::from_static(s);
        values.extend(std::iter::once(value));
    }
}

#[derive(Serialize)]
pub struct Pagination {
    pub page: u32,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/count")]
pub struct ContactsCount;

pub async fn contacts_count(
    _: ContactsCount,
    State(state): State<AppState>,
) -> Result<String, AppError> {
    let pool = state.db_pool.get().await?;
    let count: i64 = pool
        .interact(|connection| {
            use crate::schema::contacts::dsl::contacts;

            contacts.count().get_result(connection)
        })
        .await??;
    Ok(format!("({} total Contacts)", count))
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/new")]
pub struct AddContact;

pub async fn contacts_new_get(_: AddContact, flashes: IncomingFlashes) -> impl IntoResponse {
    new_contact_form(PendingContact::default(), HashMap::new(), flashes)
}

pub async fn contacts_new_post(
    _: AddContact,
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    flash: Flash,
    Form(pending_contact): Form<PendingContact>,
) -> Result<Response<Body>, AppError> {
    let contact = pending_contact.to_valid();
    if let Err(errors) = contact {
        return Ok(new_contact_form(pending_contact.clone(), errors, flashes).into_response());
    } else if let Ok(contact) = contact {
        use crate::schema::contacts;

        let connection = state.db_pool.get().await?;
        connection
            .interact(|connection| {
                diesel::insert_into(contacts::table)
                    .values(contact)
                    .returning(Contact::as_returning())
                    .execute(connection)
            })
            .await??;
    }
    Ok((
        flash.success("Created a new contact!"),
        Redirect::to(&Contacts.to_string()),
    )
        .into_response())
}

pub fn new_contact_form(
    contact: PendingContact,
    errors: HashMap<&str, String>,
    flashes: IncomingFlashes,
) -> impl IntoResponse {
    fn contact_form(
        contact: PendingContact,
        errors: HashMap<&str, String>,
    ) -> maud::PreEscaped<String> {
        let body = html! {
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
        };
        body
    }

    let body = contact_form(contact, errors);
    page(body, flashes)
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/:id")]
pub struct ViewContact {
    pub id: ContactId,
}

pub async fn find_contact(pool: Pool, contact_id: ContactId) -> Result<Contact, AppError> {
    let connection = pool.get().await?;
    let contact = connection
        .interact(move |connection| {
            use crate::schema::contacts::dsl::contacts;

            let contact: Contact = contacts
                .find(contact_id)
                .select(Contact::as_select())
                .first(connection)?;
            Ok::<Contact, AppError>(contact)
        })
        .await??
        .clone();
    Ok(contact)
}

pub async fn contacts_view(
    ViewContact { id }: ViewContact,
    State(state): State<AppState>,
    flash: Flash,
    flashes: IncomingFlashes,
) -> Result<Response<Body>, AppError> {
    let contact = find_contact(state.db_pool, id).await;
    if let Ok(contact) = contact {
        fn contact_info(contact: Contact, id: ContactId) -> maud::PreEscaped<String> {
            let body = html! {
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
            };
            body
        }
        let body = contact_info(contact, id);
        Ok(page(body, flashes).into_response())
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
pub struct UpdateContact {
    pub id: ContactId,
}

pub async fn contacts_edit_get(
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
    let contact = contact.unwrap();
    edit_contact_form(id, contact.into(), HashMap::new(), flashes).into_response()
}

pub async fn contacts_edit_post(
    UpdateContact { id }: UpdateContact,
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    flash: Flash,
    Form(pending_contact): Form<PendingContact>,
) -> Result<Response<Body>, AppError> {
    let pending = pending_contact.clone();
    let contact = pending_contact.to_valid();
    match contact {
        Err(errors) => return Ok(edit_contact_form(id, pending, errors, flashes).into_response()),
        Ok(contact) => {
            let connection = state.db_pool.get().await?;
            connection
                .interact(move |connection| {
                    use crate::schema::contacts::dsl::contacts;

                    let contact_id = id;
                    diesel::update(contacts.find(contact_id))
                        .set(contact)
                        .execute(connection)
                })
                .await??;
        }
    };
    Ok((
        flash.success("Updated contact!"),
        Redirect::to(&ViewContact { id }.to_string()),
    )
        .into_response())
}

pub fn edit_contact_form(
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
            button #(DeleteTrigger::Button.id()) hx-delete=(ViewContact{id})
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

pub enum DeleteTrigger {
    Button,
}

impl DeleteTrigger {
    fn id(&self) -> &'static str {
        match self {
            Self::Button => "delete-btn",
        }
    }
}

static HX_TRIGGER: HeaderName = HeaderName::from_static("hx-trigger");

impl axum_extra::headers::Header for DeleteTrigger {
    fn name() -> &'static axum::http::HeaderName {
        &HX_TRIGGER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum_extra::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i axum::http::HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(axum_extra::headers::Error::invalid)?;

        if value == "delete-btn" {
            Ok(DeleteTrigger::Button)
        } else {
            Err(axum_extra::headers::Error::invalid())
        }
    }

    fn encode<E: Extend<axum::http::HeaderValue>>(&self, values: &mut E) {
        let s = self.id();
        let value = HeaderValue::from_static(s);
        values.extend(std::iter::once(value));
    }
}

pub async fn contacts_delete(
    ViewContact { id: contact_id }: ViewContact,
    State(state): State<AppState>,
    flash: Flash,
    deleted_trigger: Option<TypedHeader<DeleteTrigger>>,
) -> Result<Response<Body>, AppError> {
    let connection = state.db_pool.get().await?;
    connection
        .interact(move |connection| {
            use crate::schema::contacts::dsl::contacts;

            diesel::delete(contacts.find(contact_id)).execute(connection)?;
            Ok::<(), AppError>(())
        })
        .await??;

    if matches!(deleted_trigger.as_deref(), Some(DeleteTrigger::Button)) {
        Ok((
            flash.success("Deleted contact, yo!"),
            Redirect::to(&Contacts.to_string()),
        )
            .into_response())
    } else {
        Ok("".into_response())
    }
}

#[derive(Deserialize)]
pub struct DeleteContactList {
    #[serde(default)]
    pub selected_contact_ids: Vec<ContactId>,
}

// This is already at the `Contacts` page,
// so we don't have to redirect,
// but unsure if this is what we want.
// We might want to copy things over, but
// what if we were searching or navigating through the pages?
// Would we copy all of that logic over here?
// The example in the book renders all contacts.
pub async fn contacts_delete_all(
    _: Contacts,
    State(state): State<AppState>,
    flash: Flash,
    Form(to_delete): Form<DeleteContactList>,
) -> Result<Response<Body>, AppError> {
    let connection = state.db_pool.get().await?;
    connection
        .interact(|connection| {
            use crate::schema::contacts::dsl::contacts;
            use crate::schema::contacts::dsl::id;

            diesel::delete(contacts.filter(id.eq_any(to_delete.selected_contact_ids)))
                .execute(connection)?;
            Ok::<(), AppError>(())
        })
        .await??;

    Ok((
        flash.success("Deleted contacts!"),
        Redirect::to(&Contacts.to_string()),
    )
        .into_response())
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/:id/email")]
pub struct ContactEmail {
    pub id: ContactId,
}

#[derive(Debug, Deserialize)]
pub struct EmailValidationParams {
    pub email_address: Option<String>,
}

pub async fn contacts_email_get(
    _: ContactEmail,
    Query(query): Query<EmailValidationParams>,
    State(state): State<AppState>,
) -> Result<Response<Body>, AppError> {
    let email = query.email_address.unwrap_or_default();
    if email.is_empty() {
        return Ok("Email cannot be empty".into_response());
    }

    let connection = state.db_pool.get().await?;
    let contact_count: i64 = connection
        .interact(|connection| {
            use crate::schema::contacts::dsl::contacts;
            use crate::schema::contacts::dsl::email_address;

            contacts
                .filter(email_address.like(email))
                .count()
                .get_result(connection)
        })
        .await??;
    if contact_count == 0 {
        Ok("".into_response())
    } else {
        Ok("Email must be unique".into_response())
    }
}
