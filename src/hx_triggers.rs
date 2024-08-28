use axum::http::HeaderName;
use axum::http::HeaderValue;

static HX_TRIGGER: HeaderName = HeaderName::from_static("hx-trigger");

macro_rules! form_struct {
    (#[derive( $($derive_attributes:path),* $(,)?)] $vis:vis struct $struct_name:ident { $($field:ident($rename:expr): $typ:ty),+ $(,)?}) => {
        #[allow(non_snake_case)]
        $vis mod $struct_name {
            #[allow(unused_imports)]
            use serde::Deserialize;

            #[derive($($derive_attributes, )*)]
            $vis struct Form {
                $(#[serde(rename = $rename)]
                $vis $field: $typ,)+
            }

            $($vis fn $field() -> &'static str { $rename })+

            $vis struct Errors {
                $($vis $field: Option<&'static str>,)+
            }
        }
    };
}

pub(crate) use form_struct;

macro_rules! hx_trigger_variants {
    ($enum_name:ident { $($variant:ident: $id:expr),+ }) => {
        pub enum $enum_name {
            $($variant,)+
        }
        impl $enum_name {
            pub fn id(&self) -> &'static str {
                match self {
                    $(Self::$variant => $id),+
                }
            }
        }

        impl axum_extra::headers::Header for $enum_name {
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

                $(if value == $id {
                    return Ok(Self::$variant);
                })+
                return Err(axum_extra::headers::Error::invalid())
            }

            fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
                let s = self.id();
                let value = HeaderValue::from_static(s);
                values.extend(std::iter::once(value));
            }
        }
    }
}

// Could put enum declaration outside of macro if more methods are needed.
// That would mean that we duplicate the variants.
hx_trigger_variants!(ContactsInteraction { Search: "search" });
hx_trigger_variants!(DeleteTrigger {
    Button: "delete-btn"
});
