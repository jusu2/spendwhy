//! `arch_newtype!` 声明式宏, 用于值对象类型。
//!
//! 两种形式:
//!
//! 1. **带校验器的 String newtype**: 派生 `Debug + Clone + Eq + Hash +
//!    serde::{Serialize, Deserialize} + Display + TryFrom<String>`。
//!    `serde` 实现会在反序列化时跑校验器, 所以 DTO 在系统边界即可拒绝
//!    非法值, 无需额外代码。
//!
//! 2. **Uuid newtype**: `uuid::Uuid` 的类型安全包装, 在 serde 中透明,
//!    提供 `Default` 和 `Display`。
//!
//! ## 调用方依赖
//!
//! 调用方必须在自己的 `Cargo.toml` 里有 `serde` (带 `derive`) 以及
//! (Uuid 形式所需的) `uuid`。宏使用绝对路径 (`::serde::...`,
//! `::uuid::...`), 因此 kernel 不会再导出这些 crate。
//!
//! ## 示例
//!
//! ```
//! use archforge_kernel::arch_newtype;
//!
//! arch_newtype! {
//!     /// 类 RFC 5322 邮箱。校验器在测试里刻意收紧, 同时对 fixture
//!     /// 数据足够宽松。
//!     pub struct Email(String) where |s| s.contains('@') && s.len() >= 3;
//! }
//!
//! arch_newtype! {
//!     pub struct OrderId(Uuid);
//! }
//! ```

/// 见模块级文档。
#[macro_export]
macro_rules! arch_newtype {
    // ---- 带校验器的 String newtype --------------------------------------
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
            /// 校验后构造。校验器拒绝值时返回
            /// [`archforge_kernel::AppError::Invalid`]。
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

            /// 借用内部字符串。
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// 消费并取出内部 `String`。
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
            /// 全新的随机 v4 id。
            pub fn new() -> Self {
                Self(::uuid::Uuid::new_v4())
            }

            /// 包装一个已有 uuid。
            #[allow(dead_code)]
            pub const fn from_uuid(u: ::uuid::Uuid) -> Self {
                Self(u)
            }

            /// 内部 uuid。
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

    // String 形式 ----------------------------------------------------------
    arch_newtype! {
        /// 测试用邮箱: 最低限度校验。
        pub struct Email(String) where |s| s.contains('@') && s.len() >= 3;
    }

    // Uuid 形式 ------------------------------------------------------------
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

        // 往返
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
