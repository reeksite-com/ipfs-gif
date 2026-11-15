use axum::{
    http::{ StatusCode },
    response::{ IntoResponse, Response, Redirect },
};
use askama::Template;
use garde::{
    Validate, Report,
};
use macros::{ RouteParamsContext, render_template };
use urlencoding::encode;

use crate::database;
use crate::ui_pages::moderation_queue::{ ModerationQueueTemplate };
use crate::router::{ html_to_response };
use crate::router::authn::UserAccountType;
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };
use crate::router::validation::{ create_simple_report };

#[derive(Default, RouteParamsContext)]
pub struct ModerationQueuePageParams {
    #[route_param_source(source = "none")]
    pub validation_report: Option<Report>,
}
pub type ModerationQueuePageContext = BaseContext<ModerationQueuePageParams>;

pub async fn get_moderation_queue(
    Context { context }: Context<ModerationQueuePageParams>,
) -> Response {

    if !context.user.is_some() {
        return Redirect::to(
            format!("/authenticate/?redirect-to={}", encode(context.route_original_uri.path())).as_str()
        ).into_response();
    }

    send_moderation_queue_page_response(StatusCode::OK, context).await
}

#[derive(Default, Debug, RouteParamsContext, Validate)]
pub struct PostModerationQueueReportPageParams {
    #[route_param_source(source = "path", name = "report_id", default = -1)]
    #[garde(skip)]
    pub report_id: u64,

    #[route_param_source(source = "form", name = "action", default = "")]
    #[garde(
        custom(is_valid_action(&self.action))
    )]
    pub action: String,
}

pub async fn post_moderation_queue_report(
    Context { context }: Context<PostModerationQueueReportPageParams>,
) -> Response {
    let mut page_context = context.clone_with_params(ModerationQueuePageParams {
        validation_report: None,
    });

    if let Some(user) = context.user {
        if !(user.account_type == UserAccountType::Admin || user.account_type == UserAccountType::Moderator) {
            page_context.params.validation_report = Some(
                create_simple_report("server_error".to_string(), "User does not have the correct permissions.".to_string())
            );
            return send_moderation_queue_page_response(StatusCode::BAD_REQUEST, page_context).await;
        }
    } else {
        return Redirect::to(
            format!("/authenticate/?redirect-to={}", encode("/moderation-queue/")).as_str()
        ).into_response();
    }

    let validation_result = validate_moderation_queue_report_form(&context.params).await;
    if let Err(report) = validation_result {
        page_context.params.validation_report = Some(report);
        return send_moderation_queue_page_response(StatusCode::BAD_REQUEST, page_context).await;
    }

    if context.params.action == "accept" {
        let _ = database::accept_moderation_report(context.params.report_id).await;
    } else if context.params.action == "reject" {
        let _ = database::reject_moderation_report(context.params.report_id).await;
    }

    Redirect::to("/moderation-queue/").into_response()
}

#[derive(Default, Debug, RouteParamsContext, Validate)]
pub struct PostModerationQueueReporterPageParams {
    #[route_param_source(source = "path", name = "public_key", default = -1)]
    #[garde(skip)]
    pub public_key: String,

    #[route_param_source(source = "path", name = "ip_address", default = -1)]
    #[garde(skip)]
    pub ip_address: String,

    #[route_param_source(source = "form", name = "action", default = "")]
    #[garde(
        custom(is_valid_action(&self.action))
    )]
    pub action: String,
}

pub async fn post_moderation_queue_reporter(
    Context { context }: Context<PostModerationQueueReporterPageParams>,
) -> Response {
    let mut page_context = context.clone_with_params(ModerationQueuePageParams {
        validation_report: None,
    });

    if let Some(user) = context.user {
        if !(user.account_type == UserAccountType::Admin || user.account_type == UserAccountType::Moderator) {
            page_context.params.validation_report = Some(
                create_simple_report("server_error".to_string(), "User does not have the correct permissions.".to_string())
            );
            return send_moderation_queue_page_response(StatusCode::BAD_REQUEST, page_context).await;
        }
    } else {
        return Redirect::to(
            format!("/authenticate/?redirect-to={}", encode("/moderation-queue/")).as_str()
        ).into_response();
    }

    let validation_result = validate_moderation_queue_reporter_form(&context.params).await;
    if let Err(report) = validation_result {
        page_context.params.validation_report = Some(report);
        return send_moderation_queue_page_response(StatusCode::BAD_REQUEST, page_context).await;
    }

    if context.params.action == "reject" {
        let _ = database::reject_moderation_reporter(
            &context.params.public_key,
            &context.params.ip_address,
        ).await;
    }

    Redirect::to("/moderation-queue/").into_response()
}

pub async fn send_moderation_queue_page_response(status: StatusCode, context: ModerationQueuePageContext) -> Response {
    (
        status,
        html_to_response(
            &context,
            |hx_target, context| async move {
                match hx_target.as_str() {
                    "main-article" => render_template!(ModerationQueueTemplate, &context, page_content),
                    _ => render_template!(ModerationQueueTemplate, &context),
                }
            }
        ).await
    ).into_response()
}

async fn validate_moderation_queue_report_form(form: &PostModerationQueueReportPageParams) -> Result<(), Report> {
    if let Err(report) = form.validate() {
        return Err(report);
    }
    Ok(())
}

async fn validate_moderation_queue_reporter_form(form: &PostModerationQueueReporterPageParams) -> Result<(), Report> {
    if let Err(report) = form.validate() {
        return Err(report);
    }
    Ok(())
}

fn is_valid_action<'a>(action: &'a str) -> impl FnOnce(&str, &()) -> garde::Result + 'a {
    move |_, _| {
        if action == "accept" || action == "reject" {
            Ok(())
        } else {
            Err(garde::Error::new("Invalid action."))
        }
    }
}
