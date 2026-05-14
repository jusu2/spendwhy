//! End-to-end integration test: simulate an FFI entry point that uses
//! `guard_async` around a use-case future and serialises the result to JSON
//! via `WireError`. The test asserts the contract that any host (Dart /
//! Swift / C#) would rely on:
//!
//! 1. A successful future round-trips its payload.
//! 2. A business `AppError` is preserved with the right `kind` discriminator.
//! 3. A panic inside the future appears as `kind = "internal"` with
//!    `is_panic = true` and never crashes the process.
//! 4. A panic across an `.await` point is still caught.

use archforge_ffi::{guard_async, WireError, WireErrorKind};
use archforge_kernel::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FakePayload {
    id: u32,
    label: String,
}

async fn simulated_ffi_entry(behaviour: Behaviour) -> Result<String, WireError> {
    let result = guard_async(async move {
        match behaviour {
            Behaviour::Ok => Ok(FakePayload {
                id: 1,
                label: "ok".into(),
            }),
            Behaviour::BusinessNotFound => Err(AppError::NotFound("user/42".into())),
            Behaviour::PanicImmediate => panic!("immediate panic"),
            Behaviour::PanicAfterYield => {
                tokio::task::yield_now().await;
                panic!("yielded then panicked");
            }
        }
    })
    .await;

    WireError::from_result(result).map(|payload| serde_json::to_string(&payload).unwrap())
}

#[derive(Debug, Clone, Copy)]
enum Behaviour {
    Ok,
    BusinessNotFound,
    PanicImmediate,
    PanicAfterYield,
}

#[tokio::test]
async fn ok_round_trips_payload() {
    let s = simulated_ffi_entry(Behaviour::Ok).await.unwrap();
    let payload: FakePayload = serde_json::from_str(&s).unwrap();
    assert_eq!(
        payload,
        FakePayload {
            id: 1,
            label: "ok".into()
        }
    );
}

#[tokio::test]
async fn business_error_keeps_kind() {
    let err = simulated_ffi_entry(Behaviour::BusinessNotFound)
        .await
        .unwrap_err();
    assert_eq!(err.kind, WireErrorKind::NotFound);
    assert_eq!(err.message, "user/42");
    assert!(!err.is_panic);
}

#[tokio::test]
async fn immediate_panic_is_caught_and_flagged() {
    let err = simulated_ffi_entry(Behaviour::PanicImmediate)
        .await
        .unwrap_err();
    assert_eq!(err.kind, WireErrorKind::Internal);
    assert!(err.is_panic);
    assert!(err.message.contains("immediate panic"));
}

#[tokio::test]
async fn panic_after_await_is_caught_and_flagged() {
    let err = simulated_ffi_entry(Behaviour::PanicAfterYield)
        .await
        .unwrap_err();
    assert_eq!(err.kind, WireErrorKind::Internal);
    assert!(err.is_panic);
    assert!(err.message.contains("yielded then panicked"));
}

#[tokio::test]
async fn wire_error_survives_json_round_trip() {
    let original = simulated_ffi_entry(Behaviour::BusinessNotFound)
        .await
        .unwrap_err();
    let json = serde_json::to_string(&original).unwrap();
    let parsed: WireError = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, original);
}

/// Property test: every `AppError` variant maps to a `WireError` whose
/// `is_panic` flag is true iff and only if the message starts with the
/// canonical panic tag.
#[test]
fn appserror_to_wire_panic_flag_is_sound() {
    use proptest::prelude::*;

    proptest!(|(msg in "[a-zA-Z0-9 :._]{0,40}")| {
        let panic_msg = format!("{}{}", archforge_ffi::PANIC_INTERNAL_TAG, msg);
        let plain_err = AppError::Internal(msg.clone());
        let panic_err = AppError::Internal(panic_msg.clone());

        let w_plain: WireError = plain_err.into();
        let w_panic: WireError = panic_err.into();

        prop_assert!(!w_plain.is_panic, "plain Internal({:?}) flagged as panic", msg);
        prop_assert!(w_panic.is_panic, "tagged Internal({:?}) not flagged", panic_msg);
    });
}
