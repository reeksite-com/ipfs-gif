use std::path::PathBuf;

use axum::{
    extract::{ DefaultBodyLimit },
    routing::{ get, get_service, post },
    Router,
};
use axum_governor::{
    GovernorConfigBuilder,
    GovernorLayer,
    Quota,
    nz,
    extractor::SmartIp
};
use axum_login::{
    tower_sessions::{ MemoryStore, SessionManagerLayer },
    AuthManagerLayerBuilder,
};
use tower_http::{
    services::{ ServeDir },
};
use tower_http::cors::{ CorsLayer, Any };
use super::authn::Backend;
use super::governor::PathKeyExtractor;

pub mod api;
pub mod authenticate;
pub mod counter_claim;
pub mod dcma;
pub mod explore;
pub mod gif;
pub mod home;
pub mod moderation_queue;
pub mod page_not_found;
pub mod privacy_policy;
pub mod report;
pub mod robots;
pub mod search;
pub mod tag;
pub mod terms_of_service;
pub mod trending;
pub mod upload;

pub fn initialize() -> Router {
    // Static assets
    let memory_router = memory_serve::load!()
        .into_router();

    // Uploaded assets
    let uploaded_files_path = PathBuf::from("uploads");

    let api_cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);
    
    let analytics_api_rate_limiting_config = GovernorConfigBuilder::default()
        .with_extractor(axum_governor::extractor::Compound(PathKeyExtractor::default(), SmartIp::default()))
        .expect_connect_info()
        .quota_default(Quota::requests_per_hour(nz!(1u32)))
        .finish()
        .unwrap();

    let private_api_rate_limiting_config = GovernorConfigBuilder::default()
        .with_extractor(SmartIp::default())
        .expect_connect_info()
        .quota_default(Quota::requests_per_second(nz!(60u32)))
        .finish()
        .unwrap();
    
    let public_api_rate_limiting_config = GovernorConfigBuilder::default()
        .with_extractor(SmartIp::default())
        .expect_connect_info()
        .quota_default(Quota::requests_per_second(nz!(60u32)))
        .finish()
        .unwrap();
    
    // Session layer.
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store);

    // Auth service.
    let backend = Backend::default();
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    let app = Router::new()
        .merge(memory_router)
        .route("/", get(home::get_home))

        .route("/404", get(page_not_found::get_page_not_found))
        .route("/404/", get(page_not_found::get_page_not_found))

        .route("/api", get(api::get_api))
        .route("/api/", get(api::get_api))
        .merge(
            Router::new()
                .route("/api/v1/analytics/gif/{cid}", post(api::post_api_v1_analytics_gif))
                .layer(GovernorLayer::new(analytics_api_rate_limiting_config))
        )
        .merge(
            Router::new()
                .route("/api/v1/pins", get(api::get_api_v1_pins))
                .route("/api/v1/exports/{export_offset}", get(api::get_api_v1_export))
                .route("/api/v1/quarantine/{quarantine_id}", get(api::get_api_v1_quarantine))
                .layer(GovernorLayer::new(private_api_rate_limiting_config))
        )
        .merge(
            Router::new()
                .route("/api/v1/popular", get(api::get_api_v1_popular))
                .route("/api/v1/search", get(api::get_api_v1_search))
                .route("/api/v1/tag/{tag_hash}", get(api::get_api_v1_tag))
                .layer(api_cors)
                .layer(GovernorLayer::new(public_api_rate_limiting_config))
        )

        .route("/authenticate", get(authenticate::get_authenticate))
        .route("/authenticate/", get(authenticate::get_authenticate))
        .route("/authenticate", post(authenticate::post_authenticate))
        .route("/authenticate/", post(authenticate::post_authenticate))

        .route("/counter-claim/{cid}", get(counter_claim::get_counter_claim))
        .route("/counter-claim/{cid}/", get(counter_claim::get_counter_claim))
        .route("/counter-claim/{cid}", post(counter_claim::post_counter_claim))
        .route("/counter-claim/{cid}/", post(counter_claim::post_counter_claim))

        .route("/dcma", get(dcma::get_dcma))
        .route("/dcma/", get(dcma::get_dcma))

        .route("/explore", get(explore::get_explore))
        .route("/explore/", get(explore::get_explore))

        .route("/gif/{cid}", get(gif::get_gif))
        .route("/gif/{cid}/", get(gif::get_gif))
        .route("/gif/{cid}", post(gif::post_gif))
        .route("/gif/{cid}/", post(gif::post_gif))

        .route("/moderation-queue", get(moderation_queue::get_moderation_queue))
        .route("/moderation-queue/", get(moderation_queue::get_moderation_queue))
        .route("/moderation-queue/report/{report_id}", post(moderation_queue::post_moderation_queue_report))
        .route("/moderation-queue/report/{report_id}/", post(moderation_queue::post_moderation_queue_report))
        .route("/moderation-queue/reporter/{public_key}/{ip_address}", post(moderation_queue::post_moderation_queue_reporter))
        .route("/moderation-queue/reporter/{public_key}/{ip_address}/", post(moderation_queue::post_moderation_queue_reporter))

        .route("/privacy-policy", get(privacy_policy::get_privacy_policy))
        .route("/privacy-policy/", get(privacy_policy::get_privacy_policy))

        .route("/report/{cid}", get(report::get_report))
        .route("/report/{cid}/", get(report::get_report))
        .route("/report/{cid}", post(report::post_report))
        .route("/report/{cid}/", post(report::post_report))

        .route("/robots.txt", get(robots::get_robots))

        .route("/search", get(search::get_search))
        .route("/search/", get(search::get_search))

        .route("/tag/{tag_hash}", get(tag::get_tag))
        .route("/tag/{tag_hash}/", get(tag::get_tag))

        .route("/trending", get(trending::get_trending))
        .route("/trending/", get(trending::get_trending))

        .route("/terms-of-service", get(terms_of_service::get_terms_of_service))
        .route("/terms-of-service/", get(terms_of_service::get_terms_of_service))

        .route("/upload", get(upload::get_upload))
        .route("/upload/", get(upload::get_upload))
        .route("/upload", post(upload::post_upload).layer(DefaultBodyLimit::max(1024 * 1024 * 12)))
        .route("/upload/", post(upload::post_upload).layer(DefaultBodyLimit::max(1024 * 1024 * 12)))

        .layer(auth_layer)
        .fallback_service(get_service(ServeDir::new(uploaded_files_path)));

    app
}