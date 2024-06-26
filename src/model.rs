use std::collections::HashMap;
use std::fmt::Display;

use diesel::query_builder::AsChangeset;
use diesel::Insertable;
use diesel::Queryable;
use diesel::Selectable;
use diesel_derive_newtype::DieselNewType;
use serde::Deserialize;
use serde::Serialize;

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

/// Pending contact that is the information entered by the user. Could be
/// missing fields or have invalid fields (eg, bogus email address format).
/// Could experiment with just using a HashMap for the next endpoint.
#[derive(Deserialize, Default, Debug, Clone)]
pub struct PendingContact {
    #[serde(deserialize_with = "non_empty_str")]
    pub first_name: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub last_name: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub phone: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub email_address: Option<String>,
}

#[derive(Insertable, AsChangeset, Deserialize)]
#[diesel(table_name = crate::schema::contacts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewContact {
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub email_address: String,
}

pub(crate) fn non_empty_str<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<String>, D::Error> {
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
    pub fn to_valid(&self) -> Result<NewContact, HashMap<&'static str, String>> {
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
                let mut errors = HashMap::new();

                if self.first_name.is_none() {
                    errors.insert("first", "Missing first name".into());
                }
                if self.last_name.is_none() {
                    errors.insert("last", "Missing last name".into());
                }
                if self.phone.is_none() {
                    errors.insert("phone", "Missing phone".into());
                }
                if self.email_address.is_none()
                    || self.email_address.as_ref().is_some_and(|s| s.is_empty())
                {
                    errors.insert("email", "Missing email address".into());
                }

                Err(errors)
            }
        }
    }
}
