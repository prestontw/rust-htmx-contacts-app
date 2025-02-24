use axum::http::HeaderName;

pub(crate) static HX_TRIGGER: HeaderName = HeaderName::from_static("hx-trigger");

// Could put enum declaration outside of macro if more methods are needed.
// That would mean that we duplicate the variants.
#[macro_export]
macro_rules! hx_trigger_variants {
    ($enum_name:ident {
        $($variant:ident: $id:expr),+
    }) => {
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
                &$crate::hx_triggers::HX_TRIGGER
            }

            fn decode<'i, I>(values: &mut I) -> Result<Self, axum_extra::headers::Error>
            where
                Self: Sized,
                I: Iterator<Item = &'i axum::http::HeaderValue>,
            {
                let value = values
                    .next()
                    .ok_or_else(axum_extra::headers::Error::invalid)?;

                $(if value == $id {
                    return Ok(Self::$variant);
                })+
                return Err(axum_extra::headers::Error::invalid())
            }

            fn encode<E: Extend<axum::http::HeaderValue>>(&self, values: &mut E) {
                let s = self.id();
                let value = axum::http::HeaderValue::from_static(s);
                values.extend(std::iter::once(value));
            }
        }
    }
}
