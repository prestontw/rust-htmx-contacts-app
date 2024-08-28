use std::fmt::Display;

use diesel::query_builder::AsChangeset;
use diesel::Insertable;
use diesel::Queryable;
use diesel::Selectable;
use diesel_derive_newtype::DieselNewType;
use serde::Deserialize;
use serde::Serialize;

use crate::hx_triggers::form_struct;

#[derive(DieselNewType, Clone, Copy, Debug, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ContactId(i32);

impl Display for ContactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(AsChangeset, Queryable, Selectable, Clone, Debug, Deserialize, Serialize)]
#[diesel(table_name = crate::schema::contacts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Contact {
    pub id: ContactId,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub email_address: String,
}

form_struct!(
#[derive(serde::Deserialize, Default, Debug, Clone)]
pub struct PendingContact {
     first_name("first_name"): Option<String>,
     last_name("last_name"): Option<String>,
     phone("phonee"): Option<String>,
     email_address("email_address"): Option<String>,
}
);

#[derive(Insertable, AsChangeset, Deserialize)]
#[diesel(table_name = crate::schema::contacts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewContact {
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub email_address: String,
}

impl From<Contact> for PendingContact::Form {
    fn from(value: Contact) -> Self {
        Self {
            first_name: Some(value.first_name),
            last_name: Some(value.last_name),
            phone: Some(value.phone),
            email_address: Some(value.email_address),
        }
    }
}

impl PendingContact::Form {
    pub fn to_valid(&self) -> Result<NewContact, PendingContact::Errors> {
        match (
            &self.first_name,
            &self.last_name,
            &self.phone,
            &self.email_address,
        ) {
            (Some(first_name), Some(last_name), Some(phone), Some(email)) if !email.is_empty() => {
                Ok(NewContact {
                    first_name: first_name.to_string(),
                    last_name: last_name.to_string(),
                    phone: phone.to_string(),
                    email_address: email.to_string(),
                })
            }
            _ => {
                let mut errors = PendingContact::Errors::default();

                if self.first_name.is_none() {
                    errors.first_name = Some("Missing first name");
                }
                if self.last_name.is_none() {
                    errors.last_name = Some("Missing last name");
                }
                if self.phone.is_none() {
                    errors.phone = Some("Missing phone");
                }
                if self.email_address.is_none()
                    || self.email_address.as_ref().is_some_and(|s| s.is_empty())
                {
                    errors.email_address = Some("Missing email address");
                }

                Err(errors)
            }
        }
    }
}
