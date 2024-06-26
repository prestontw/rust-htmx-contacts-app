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

use crate::html_views::Contacts;
use crate::html_views::ViewContact;
use crate::model::Contact;
use crate::model::NewContact;
use crate::AppError;
use crate::AppState;

pub async fn get_contacts(
    _: Contacts,
    State(state): State<AppState>,
) -> Result<Response<Body>, AppError> {
    #[derive(Serialize)]
    struct Contacts {
        contacts: Vec<Contact>,
    }

    let connection = state.db_pool.get().await?;
    let contacts: Vec<Contact> = connection
        .interact(|connection| {
            use crate::schema::contacts::dsl::*;

            contacts
                .select(Contact::as_select())
                .get_results(connection)
        })
        .await??;

    Ok(Json(Contacts { contacts }).into_response())
}

pub async fn get_contact(
    ViewContact { id: contact_id }: ViewContact,
    State(state): State<AppState>,
) -> Result<Response<Body>, AppError> {
    let connection = state.db_pool.get().await?;
    let contact: Option<Contact> = connection
        .interact(move |connection| {
            use crate::schema::contacts::dsl::*;

            contacts.find(contact_id).first(connection).optional()
        })
        .await??;
    match contact {
        None => Ok((StatusCode::NOT_FOUND, "Could not find contact").into_response()),
        Some(contact) => Ok(Json(contact).into_response()),
    }
}

pub async fn update_contact(
    ViewContact { id: contact_id }: ViewContact,
    State(state): State<AppState>,
    Json(contact): Json<Contact>,
) -> Result<Response<Body>, AppError> {
    let connection = state.db_pool.get().await?;
    let contact = connection
        .interact(move |connection| {
            use crate::schema::contacts::dsl::*;

            diesel::update(contacts.find(contact_id))
                .set(contact)
                .returning(Contact::as_returning())
                .get_result(connection)
        })
        .await??;
    Ok(Json(contact).into_response())
}

pub async fn delete_contact(
    ViewContact { id: contact_id }: ViewContact,
    State(state): State<AppState>,
) -> Result<Response<Body>, AppError> {
    let connection = state.db_pool.get().await?;
    connection
        .interact(move |connection| {
            use crate::schema::contacts::dsl::*;

            diesel::delete(contacts.find(contact_id)).execute(connection)?;
            Ok::<(), AppError>(())
        })
        .await??;
    Ok((StatusCode::OK, "Successfully deleted").into_response())
}

pub async fn new_contact(
    _: Contacts,
    State(state): State<AppState>,
    Json(new_contact): Json<NewContact>,
) -> Result<Json<Contact>, AppError> {
    let connection = state.db_pool.get().await?;
    let new_contact = connection
        .interact(|connection| {
            use crate::schema::contacts;

            diesel::insert_into(contacts::table)
                .values(new_contact)
                .returning(Contact::as_returning())
                .get_result(connection)
        })
        .await??;
    Ok(Json(new_contact))
}
