use axum::{
    body::Body,
    http::{ header, StatusCode },
    response::{ Response, IntoResponse },
    Json,
};
use askama::Template;
use macros::{ RouteParamsContext, render_template };
use serde::Serialize;

use crate::database::{ self, QuarantineScanResult };
use crate::ui_pages::api::{ ApiTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };

#[derive(Default, RouteParamsContext)]
pub struct ApiPageParams {
}
pub type ApiPageContext = BaseContext<ApiPageParams>;

pub async fn get_api(
    Context { context }: Context<ApiPageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(ApiTemplate, &context, page_content),
                _ => render_template!(ApiTemplate, &context),
            }
        }
    ).await
}

#[derive(Default, RouteParamsContext)]
pub struct ApiV1SearchParams {
    #[route_param_source(source = "query", name = "q", default = "")]
    pub query: String,
}

#[derive(Serialize)]
pub struct ApiV1SearchResponseGif {
    pub cid: String,
    pub filename: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct ApiV1SearchResponse {
    pub media: Vec<ApiV1SearchResponseGif>,
}

pub async fn get_api_v1_search(
    Context { context }: Context<ApiV1SearchParams>,
) -> Json<ApiV1SearchResponse> {
    let gifs = match database::search_by_tags(&context.params.query, 60).await {
        Ok(gifs) => gifs,
        Err(_) => Vec::new(),
    };

    Json(ApiV1SearchResponse {
        media: gifs.into_iter().map(|gif| ApiV1SearchResponseGif {
            cid: gif.cid.unwrap_or_else(|| String::from("")),
            filename: gif.filename,
            description: gif.description,
        }).collect(),
    })
}

#[derive(Default, RouteParamsContext)]
pub struct ApiV1TagParams {
    #[route_param_source(source = "path", name = "tag_hash", default = "")]
    pub tag_hash: String,
}

pub async fn get_api_v1_tag(
    Context { context }: Context<ApiV1TagParams>,
) -> Json<ApiV1SearchResponse> {
    let split_regex = regex::Regex::new(r"[- ]").unwrap();
    let tag_name = split_regex.split(&context.params.tag_hash)
        .collect::<Vec<&str>>()
        .join(" ");

    let gifs = match database::get_gifs_by_tag(&tag_name, 0, 60).await {
        Ok(gifs) => gifs,
        Err(_) => Vec::new(),
    };

    Json(ApiV1SearchResponse {
        media: gifs.into_iter().map(|gif| ApiV1SearchResponseGif {
            cid: gif.cid.unwrap_or_else(|| String::from("")),
            filename: gif.filename,
            description: gif.description,
        }).collect(),
    })
}

#[derive(Default, RouteParamsContext)]
pub struct ApiV1PopularParams {
}

pub async fn get_api_v1_popular(
    _: Context<ApiV1PopularParams>,
) -> Json<ApiV1SearchResponse> {
    let gifs = match database::get_popular_gifs(0, 60).await {
        Ok(gifs) => gifs,
        Err(_) => Vec::new(),
    };

    Json(ApiV1SearchResponse {
        media: gifs.into_iter().map(|gif| ApiV1SearchResponseGif {
            cid: gif.cid.unwrap_or_else(|| String::from("")),
            filename: gif.filename,
            description: gif.description,
        }).collect(),
    })
}

#[derive(Default, RouteParamsContext)]
pub struct ApiV1QuarantineParams {
    #[route_param_source(source = "path", name = "quarantine_id", default = "")]
    pub quarantine_id: String,
}
#[derive(Serialize)]
pub struct ApiV1QuarantineResponse {
    cid: Option<String>,
    status: QuarantineScanResult,
}

pub async fn get_api_v1_quarantine(
    Context { context }: Context<ApiV1QuarantineParams>,
) -> Response {
    let gif = match database::get_gif_by_cid(&context.params.quarantine_id).await {
        Ok(gif) => gif,
        Err(_) => {
            return (StatusCode::NOT_FOUND).into_response();
        }
    };

    Json(ApiV1QuarantineResponse {
        cid: gif.cid,
        status: gif.quarantine_scan_result,
    }).into_response()
}

#[derive(Default, RouteParamsContext)]
pub struct ApiV1AnalyticsGifParams {
    #[route_param_source(source = "path", name = "cid", default = "")]
    pub cid: String,
}
#[derive(Serialize)]
pub struct ApiV1AnalyticsGifResponse {}

pub async fn post_api_v1_analytics_gif(
    Context { context }: Context<ApiV1AnalyticsGifParams>,
) -> Response {
    if let Err(_) = database::increment_gif_popularity(context.params.cid).await {
        return (StatusCode::BAD_REQUEST).into_response();
    }
    Json(ApiV1AnalyticsGifResponse {}).into_response()
}

#[derive(Default, RouteParamsContext)]
pub struct ApiV1PinsParams {}

pub async fn get_api_v1_pins(
    Context { context: _ }: Context<ApiV1PinsParams>,
) -> Response {
    let cids = match database::get_all_gif_cids().await {
        Ok(cids) => cids,
        Err(_) => {
            return (StatusCode::NOT_FOUND).into_response();
        }
    };

    Response::builder()
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(cids.join("\n")))
        .unwrap()
}

#[derive(Default, RouteParamsContext)]
pub struct ApiV1ExportParams {
    #[route_param_source(source = "path", name = "export_offset", default = 0)]
    pub export_offset: u64,
}

pub async fn get_api_v1_export(
    Context { context }: Context<ApiV1ExportParams>,
) -> Response {
    let gifs = match database::export_all_gifs(&context.params.export_offset * 1000, 1000).await {
        Ok(gifs) => gifs,
        Err(error) => {
            tracing::error!("Export error {:?}", error);
            return (StatusCode::NOT_FOUND).into_response();
        }
    };

    Json(
        gifs
    ).into_response()
}