use shabti_core::error::ShabtiError;
use shabti_core::health::{ComponentStatus, HealthCheck, HealthStatus};

// --- 6-4a: Production error handling and health checks ---

#[test]
fn error_has_protocol_variant() {
    let err = ShabtiError::Protocol("invalid JSON-RPC".to_string());
    assert!(format!("{err}").contains("protocol error"));
}

#[test]
fn error_has_timeout_variant() {
    let err = ShabtiError::Timeout("Qdrant query exceeded 5s".to_string());
    assert!(format!("{err}").contains("timeout"));
}

#[test]
fn error_has_config_variant() {
    let err = ShabtiError::Config("missing API key".to_string());
    assert!(format!("{err}").contains("configuration error"));
}

#[test]
fn health_check_all_healthy() {
    let check = HealthCheck::new()
        .component("storage", ComponentStatus::Healthy)
        .component("index", ComponentStatus::Healthy)
        .component("embedding", ComponentStatus::Healthy);

    assert_eq!(check.status(), HealthStatus::Healthy);
    assert!(check.is_healthy());
    assert_eq!(check.components().len(), 3);
}

#[test]
fn health_check_degraded_when_one_unhealthy() {
    let check = HealthCheck::new()
        .component("storage", ComponentStatus::Healthy)
        .component(
            "index",
            ComponentStatus::Unhealthy("connection refused".into()),
        )
        .component("embedding", ComponentStatus::Healthy);

    assert_eq!(check.status(), HealthStatus::Degraded);
    assert!(!check.is_healthy());
}

#[test]
fn health_check_unhealthy_when_critical_down() {
    let check = HealthCheck::new()
        .component("storage", ComponentStatus::Unhealthy("disk full".into()))
        .component("index", ComponentStatus::Unhealthy("unreachable".into()));

    assert_eq!(check.status(), HealthStatus::Unhealthy);
}

#[test]
fn health_check_to_json() {
    let check = HealthCheck::new()
        .component("storage", ComponentStatus::Healthy)
        .component("index", ComponentStatus::Healthy);

    let json = check.to_json();
    assert_eq!(json["status"], "healthy");
    assert!(json["components"].is_object());
    assert_eq!(json["components"]["storage"]["status"], "healthy");
}

#[test]
fn health_check_degraded_json() {
    let check = HealthCheck::new()
        .component("storage", ComponentStatus::Healthy)
        .component("index", ComponentStatus::Unhealthy("timeout".into()));

    let json = check.to_json();
    assert_eq!(json["status"], "degraded");
    assert_eq!(json["components"]["index"]["status"], "unhealthy");
    assert_eq!(json["components"]["index"]["message"], "timeout");
}

#[test]
fn component_status_display() {
    let healthy = ComponentStatus::Healthy;
    let unhealthy = ComponentStatus::Unhealthy("down".into());

    assert_eq!(format!("{healthy}"), "healthy");
    assert!(format!("{unhealthy}").contains("unhealthy"));
}
