use axum::{routing::get, routing::post, Router};

use crate::{notification_handlers, state::AppState};

pub fn notification_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/notification-templates",
            post(notification_handlers::create_notification_template),
        )
        .route(
            "/api/notification-templates/:name",
            get(notification_handlers::get_notification_template),
        )
        .route(
            "/api/users/:id/notification-preferences",
            post(notification_handlers::create_user_notification_preference)
                .get(notification_handlers::get_user_notification_preferences),
        )
        .route(
            "/api/notifications/send",
            post(notification_handlers::send_notification),
        )
        .route(
            "/api/notifications/batch",
            post(notification_handlers::send_batch_notifications),
        )
        .route(
            "/api/notifications/batch/:id",
            get(notification_handlers::get_batch_notification_status),
        )
        .route(
            "/api/notifications/batch/deliveries/:id/read",
            post(notification_handlers::mark_batch_notification_read),
        )
        .route(
            "/api/users/:id/notifications",
            get(notification_handlers::get_user_notifications),
        )
}
