#[macro_export]
macro_rules! discriminant_to_literal {
    (String, $discriminant:expr) => {
        &*$discriminant
    };
    ($discriminant_type:ident, $discriminant:expr) => {
        $discriminant
    };
}
#[macro_export]
macro_rules! def_enum {
    (
        $ident:ident ($discriminant_type:ident) {
            $(
                $discriminant:literal = $variant:ident
                $(
                    {
                        $(
                            $field:ident $typ:ident $(<$generics:ident>)?
                        );* $(;)?
                    }
                )?
            ),* $(,)?
        }
    ) => {
        use crate::discriminant_to_literal;
        use anyhow::bail;
        #[derive(Debug, Clone, Finalize, Trace, PartialEq)]
        pub enum $ident {
            $(
                $variant
                $(
                    {
                        $(
                            $field: $typ $(<$generics>)?,
                        )*
                    }
                )?,
            )*
        }
        impl $ident {
            pub fn from_num(discriminant: $discriminant_type) -> anyhow::Result<Self> {
                match discriminant_to_literal!($discriminant_type, discriminant) {
                    $(
                        $discriminant => {
                            $(
                                $(
                                    let $field = <$typ $(<$generics>)?>::read(buffer, version)
                                        .context(concat!("failed to read field `", stringify!($field),
                                            "` of enum `", stringify!($ident), "::", stringify!($variant), "`"))?
                                            .into();
                                )*
                            )?

                            Ok($ident::$variant $(
                                {
                                    $(
                                        $field,
                                    )*
                                }
                            )?)
                        },
                    )*
                    _ => bail!("No discriminant for val {}", discriminant)
                }
            }
        }
    };
}
