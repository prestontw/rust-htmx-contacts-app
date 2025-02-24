#[macro_export]
macro_rules! form_struct {
    (#[derive( $($derive_attributes:path),* $(,)?)]
     $vis:vis struct $struct_name:ident {
         $( $(#[$field_macro:tt($($params:path),* $(,)?)])*
         $field:ident($rename:expr): $typ:ty),+ $(,)?
     }) => {
        #[allow(non_snake_case)]
        $vis mod $struct_name {
            #[allow(unused_imports)]
            use serde::Deserialize;

            #[derive($($derive_attributes, )*)]
            $vis struct Form {
                $($(#[$field_macro($($params,)*)])*)*
                $(#[serde(rename = $rename)]
                $vis $field: $typ,)+
            }

            $($vis fn $field() -> &'static str { $rename })+

            #[derive(Default)]
            $vis struct Errors {
                $($vis $field: Option<&'static str>,)+
            }
        }
    };
}
