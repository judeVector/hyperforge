use http_body_util::{BodyExt, Full, Limited};
use hyper::{
    Method, Request, Response, StatusCode,
    body::{Bytes, Incoming},
    header,
};
use std::convert::Infallible;
use tracing::{error, info, warn};

use crate::{
    AppState,
    model::{create_user, delete_user, get_all_users, get_user_by_id},
};

pub async fn handle_request(
    req: Request<Incoming>,
    state: AppState,
) -> Result<Response<Full<Bytes>>, Infallible> {
    state.metrics.record_request();

    let response = match (req.method(), req.uri().path()) {
        // Health check endpoint
        (&Method::GET, "/health") => {
            info!("Health check requested");

            // Check database connectivity
            let db_healthy = sqlx::query("SELECT 1")
                .fetch_one(&state.db_pool)
                .await
                .is_ok();

            if db_healthy {
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::from(
                        r#"{"status":"healthy","database":"connected"}"#,
                    )))
                    .unwrap()
            } else {
                state.metrics.record_error();

                Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Full::new(Bytes::from(
                        r#"{"status":"unhealthy","database":"disconnected"}"#,
                    )))
                    .unwrap()
            }
        }

        // Get all users
        (&Method::GET, "/users") => match get_all_users(&state.db_pool).await {
            Ok(users) => {
                let json = serde_json::to_string(&users).unwrap();
                info!("Fetched {} users", users.len());

                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(json)))
                    .unwrap()
            }
            Err(e) => {
                state.metrics.record_error();
                error!("Database error fetching users: {}", e);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from(
                        r#"{"error":"Failed to fetch users"}"#,
                    )))
                    .unwrap()
            }
        },

        // Get user by ID
        (&Method::GET, path) if path.starts_with("/users/") => {
            let id = path.trim_start_matches("/users/").parse::<i32>();

            match id {
                Ok(user_id) => match get_user_by_id(&state.db_pool, user_id).await {
                    Ok(user) => {
                        let json = serde_json::to_string(&user).unwrap();
                        info!("Fetched user {}", user_id);

                        Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(json)))
                            .unwrap()
                    }
                    Err(sqlx::Error::RowNotFound) => {
                        warn!("User {} not found", user_id);

                        Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Full::new(Bytes::from(r#"{"error":"User not found"}"#)))
                            .unwrap()
                    }
                    Err(e) => {
                        state.metrics.record_error();
                        error!("Database error for user {}: {}", user_id, e);

                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Full::new(Bytes::from(r#"{"error":"Database error"}"#)))
                            .unwrap()
                    }
                },
                Err(_) => Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from(r#"{"error":"Invalid user ID"}"#)))
                    .unwrap(),
            }
        }

        // Create new user
        (&Method::POST, "/users") => {
            let limited_body = Limited::new(req.into_body(), 1024 * 64); // 64 KB max
            let body_bytes = match limited_body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => {
                    return Ok(Response::builder()
                        .status(StatusCode::PAYLOAD_TOO_LARGE)
                        .body(Full::new(Bytes::from(r#"{"error":"Payload too large"}"#)))
                        .unwrap());
                }
            };

            let new_user = match serde_json::from_slice(&body_bytes) {
                Ok(user) => user,
                Err(e) => {
                    error!("Failed to parse JSON: {}", e);
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Full::new(Bytes::from(r#"{"error":"Invalid JSON format"}"#)))
                        .unwrap());
                }
            };

            match create_user(&state.db_pool, new_user).await {
                Ok(user) => {
                    let json = serde_json::to_string(&user).unwrap();
                    info!("Created user {}", user.id);
                    Response::builder()
                        .status(StatusCode::CREATED)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(json)))
                        .unwrap()
                }
                Err(e) => {
                    state.metrics.record_error();
                    error!("Failed to create user: {}", e);

                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::new(Bytes::from(
                            r#"{"error":"Failed to create user"}"#,
                        )))
                        .unwrap()
                }
            }
        }

        // Delete user
        (&Method::DELETE, path) if path.starts_with("/users/") => {
            let id = path.trim_start_matches("/users/").parse::<i32>();

            match id {
                Ok(user_id) => match delete_user(&state.db_pool, user_id).await {
                    Ok(deleted) => {
                        if deleted {
                            info!("Deleted user {}", user_id);

                            Response::builder()
                                .status(StatusCode::NO_CONTENT)
                                .body(Full::new(Bytes::from(r#"{User Deleted}"#)))
                                .unwrap()
                        } else {
                            Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Full::new(Bytes::from(r#"{"error":"User not found"}"#)))
                                .unwrap()
                        }
                    }
                    Err(e) => {
                        state.metrics.record_error();
                        error!("Failed to delete user {}: {}", user_id, e);

                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Full::new(Bytes::from(
                                r#"{"error":"Failed to delete user"}"#,
                            )))
                            .unwrap()
                    }
                },
                Err(_) => Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from(r#"{"error":"Invalid user ID"}"#)))
                    .unwrap(),
            }
        }

        // Metrics endpoint
        (&Method::GET, "/metrics") => {
            let stats = state.metrics.get_stats();
            let json = serde_json::to_string(&stats).unwrap();
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }

        // 404 for everything else
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from(r#"{"error":"Not found"}"#)))
            .unwrap(),
    };

    Ok(response)
}

pub async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}
