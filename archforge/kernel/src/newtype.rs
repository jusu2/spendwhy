//! `arch_newtype!` declarative macro for value-object types.
//!
//! Two forms:
//!
//! 1. **String newtype with validator**: emits `Debug + Clone + Eq + Hash +
//!    serde::{Serialize, Deserialize} + Display + TryFrom<String>`. The
//!    `serde` impl runs the validator on deserialisation, so DTOs reject
//!    invalid values at the system boundary with no extra code.
//!
//! 2. **Uuid newtype**: type-safe wrapper around `uuid::Uuid`, transparent in
//!    serde, with `Default` and `Display`.
//!
//! ## Caller dependencies
//!
//! Callers must have `serde` (with `derive`) and (for the Uuid form) `uuid`
//! in their own `Cargo.toml`. The macro uses absolute paths (`::serde::...`,
//! `::uuid::...`) so the kernel doesn't re-export those crates.
//!
//! ## Examples
//!
//! ```
//! use archforge_kernel::arch_newtype;
//!
//! arch_newtype! {
//!     /// RFC 5322-ish email. Validator is intentionally tight in tests
//!     /// and loose enough for fixture data.
//!     pub struct Email(String) where |s| s.contains('@') && s.len() >= 3;
//! }
//!
//! arch_newtype! {
//!     pub struct OrderId(Uuid);
//! }
//! ```

/// See module-level docs.
#[macro_export]
macro_rules! arch_newtype {
    // ---- String newtype with validator -----------------------------------
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident(String) where |$arg:ident| $validate:expr;
    ) => {
        $(#[$meta])*
        #[derive(
            ::core::fmt::Debug,
            ::core::clone::Clone,
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::core::hash::Hash,
            ::serde::Serialize,
            ::serde::Deserialize,
        )]
        #[serde(try_from = "::std::string::String", into = "::std::string::String")]
        $vis struct $name(::std::string::String);

        impl $name {
            /// Construct after validation. Returns [`archforge_kernel::AppError::Invalid`]
            /// when the validator rejects the value.
            pub fn new<S>(value: S) -> ::core::result::Result<Self, $crate::AppError>
            where
                S: ::core::convert::Into<::std::string::String>,
            {
                let $arg: ::std::string::String = value.into();
                let ok: bool = { $validate };
                if ok {
                    ::core::result::Result::Ok(Self($arg))
                } else {
                    ::core::result::Result::Err($crate::AppError::Invalid(
                        ::std::format!("{}: validation failed", ::core::stringify!($name)),
                    ))
                }
            }

            /// Borrow the inner string.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume into the inner `String`.
            pub fn into_inner(self) -> ::std::string::String {
                self.0
            }
        }

        impl ::core::convert::TryFrom<::std::string::String> for $name {
            type Error = $crate::AppError;
            fn try_from(v: ::std::string::String) -> ::core::result::Result<Self, Self::Error> {
                Self::new(v)
            }
        }

        impl ::core::convert::From<$name> for ::std::string::String {
            fn from(v: $name) -> ::std::string::String {
                v.0
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };

    // ---- Uuid newtype ----------------------------------------------------
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident(Uuid);
    ) => {
        $(#[$meta])*
        #[derive(
            ::core::fmt::Debug,
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::core::hash::Hash,
            ::core::cmp::PartialOrd,
            ::core::cmp::Ord,
            ::serde::Serialize,
            ::serde::Deserialize,
        )]
        #[serde(transparent)]
        $vis struct $name(::uuid::Uuid);

        impl $name {
            /// Fresh, random v4 id.
            pub fn new() -> Self {
                Self(::uuid::Uuid::new_v4())
            }

            /// Wrap an existing uuid.
            #[allow(dead_code)]
            pub const fn from_uuid(u: ::uuid::Uuid) -> Self {
                Self(u)
            }

            /// Inner uuid.
            #[allow(dead_code)]
            pub const fn as_uuid(&self) -> ::uuid::Uuid {
                self.0
            }
        }

        impl ::core::default::Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::AppError;

    // String form ----------------------------------------------------------
    arch_newtype! {
        /// Test email: bare-minimum validation.
        pub struct Email(String) where |s| s.contains('@') && s.len() >= 3;
    }

    // Uuid form ------------------------------------------------------------
    arch_newtype! {
        pub struct OrderId(Uuid);
    }

    #[test]
    fn email_accepts_valid_input() {
        let e = Email::new("a@b").unwrap();
        assert_eq!(e.as_str(), "a@b");
        assert_eq!(e.to_string(), "a@b");
    }

    #[test]
    fn email_rejects_invalid_input() {
        let err = Email::new("nope").unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)));
    }

    #[test]
    fn email_serde_validates_on_deserialize() {
        let bad = serde_json::from_str::<Email>(r#""no-at""#);
        assert!(bad.is_err(), "deserialize should fail on invalid input");

        let good: Email = serde_json::from_str(r#""a@b.c""#).unwrap();
        assert_eq!(good.as_str(), "a@b.c");

        // Round-trip
        let json = serde_json::to_string(&good).unwrap();
        assert_eq!(json, r#""a@b.c""#);
    }

    #[test]
    fn order_id_is_transparent() {
        let id = OrderId::new();
        let json = serde_json::to_string(&id).unwrap();
        let back: OrderId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn order_id_default_is_random() {
        let a = OrderId::default();
        let b = OrderId::default();
        assert_ne!(a, b);
    }
}
