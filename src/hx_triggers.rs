use axum::http::HeaderName;
use axum::http::HeaderValue;

pub enum ContactsInteraction {
    Search,
}

impl ContactsInteraction {
    pub fn id(&self) -> &'static str {
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
        I: Iterator<Item = &'i HeaderValue>,
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

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let s = self.id();
        let value = HeaderValue::from_static(s);
        values.extend(std::iter::once(value));
    }
}

pub enum DeleteTrigger {
    Button,
}

impl DeleteTrigger {
    pub fn id(&self) -> &'static str {
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
        I: Iterator<Item = &'i HeaderValue>,
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

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let s = self.id();
        let value = HeaderValue::from_static(s);
        values.extend(std::iter::once(value));
    }
}
