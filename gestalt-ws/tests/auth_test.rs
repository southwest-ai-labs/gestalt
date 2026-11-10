use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    routing::get,
    Router,
};
use gestalt_ws::auth::{TokenValidator, ValidateAuth};
use std::env;
use std::sync::Mutex;
use tower::ServiceExt; // for `oneshot`

static TEST_MUTEX: Mutex<()> = Mutex::new(());

async fn test_handler(_auth: ValidateAuth) -> &'static str {
    "ok"
}

fn app() -> Router {
    Router::new().route("/", get(test_handler))
}

#[tokio::test]
async fn test_auth_bypassed_when_token_unset() {
    let _lock = TEST_MUTEX.lock().unwrap();
    env::remove_var("GESTALT_BUS_TOKEN");
    
    let response = app()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_rejected_when_token_configured_but_missing() {
    let _lock = TEST_MUTEX.lock().unwrap();
    env::set_var("GESTALT_BUS_TOKEN", "secret-token");
    
    let response = app()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_accepted_with_valid_bearer_token() {
    let _lock = TEST_MUTEX.lock().unwrap();
    env::set_var("GESTALT_BUS_TOKEN", "secret-token");
    
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("Authorization", "Bearer secret-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_accepted_with_valid_query_token() {
    let _lock = TEST_MUTEX.lock().unwrap();
    env::set_var("GESTALT_BUS_TOKEN", "secret-token");
    
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/?token=secret-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_rejected_with_invalid_token() {
    let _lock = TEST_MUTEX.lock().unwrap();
    env::set_var("GESTALT_BUS_TOKEN", "secret-token");
    
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("Authorization", "Bearer wrong-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_token_validator_constant_time() {
    assert!(TokenValidator::validate("abc", "abc"));
    assert!(!TokenValidator::validate("abc", "abd"));
    assert!(!TokenValidator::validate("abc", "ab"));
    assert!(!TokenValidator::validate("abc", "abcd"));
}
