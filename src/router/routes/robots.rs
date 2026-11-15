use axum::{
    http::{ header, StatusCode },
    response::{ IntoResponse },
};

pub async fn get_robots() -> impl IntoResponse {
    let body = format!("{}\n{}",
        "User-agent: *",
        "Disallow: /api/v1/",
    );

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/plain"),
        ],
        body,
    )
}
