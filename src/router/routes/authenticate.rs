use axum::{
    http::{ StatusCode },
    response::{ IntoResponse, Response, Redirect },
};
use askama::Template;
use garde::{
    Validate, Report,
};
use macros::{ RouteParamsContext, render_template };

use crate::ui_pages::authenticate::{ AuthenticateTemplate };
use crate::router::{ html_to_response };
use crate::router::authn::{ AuthSession, Credentials, ModeratorSession };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };
use crate::router::validation::{ create_simple_report };

#[derive(Default, RouteParamsContext)]
pub struct AuthenticatePageParams {
    #[route_param_source(source = "query", name="redirect-to", default = "/")]
    pub redirect_to: String,

    #[route_param_source(source = "none")]
    pub validation_report: Option<Report>,

    #[route_param_source(source = "none")]
    pub authenticate_public_key: String,
}
pub type AuthenticatePageContext = BaseContext<AuthenticatePageParams>;

pub async fn get_authenticate(
    Context { context }: Context<AuthenticatePageParams>,
) -> Response {
    send_authenticate_page_response(StatusCode::OK, context).await
}

#[derive(Default, Debug, RouteParamsContext, Validate)]
pub struct PostAuthenticatePageParams {
    #[route_param_source(source = "form", name = "redirect-to", default = "/")]
    #[garde(
        length(max = 512),
    )]
    pub redirect_to: String,

    #[route_param_source(source = "form", name = "authenticate-public-key", default = "")]
    #[garde(
        length(max = 64),
    )]
    pub authenticate_public_key: String,

    #[route_param_source(source = "form", name = "authenticate-signed-message", default = "")]
    #[garde(
        length(max = 128),
    )]
    pub authenticate_signed_message: String,
}

#[axum::debug_handler]
pub async fn post_authenticate(
    mut auth_session: AuthSession,
    Context { context }: Context<PostAuthenticatePageParams>,
) -> Response {
    let mut page_context = context.clone_with_params(AuthenticatePageParams {
        validation_report: None,
        redirect_to: context.params.redirect_to.clone(),
        authenticate_public_key: context.params.authenticate_public_key.clone(),
    });

    let validation_result = validate_authenticate_form(&context.params).await;
    if let Err(report) = validation_result {
        page_context.params.validation_report = Some(report);
        return send_authenticate_page_response(StatusCode::BAD_REQUEST, page_context).await;
    }

    if context.params.authenticate_signed_message.len() == 0 {
        return send_authenticate_page_response(StatusCode::OK, page_context).await;
    }

    let credentials = Credentials {
        public_key: context.params.authenticate_public_key.clone(),
        signed_message: context.params.authenticate_signed_message.clone(),
        ip_address: context.ip_address.clone(),
    };
    let mut status: StatusCode = StatusCode::OK;
    let user = match auth_session.authenticate(credentials).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            status = StatusCode::UNAUTHORIZED;
            ModeratorSession::default()
        },
        Err(_) => {
            status = StatusCode::INTERNAL_SERVER_ERROR;
            ModeratorSession::default()
        },
    };
    if status == StatusCode::OK && auth_session.login(&user).await.is_err() {
        status = StatusCode::INTERNAL_SERVER_ERROR;
    }
    if status != StatusCode::OK {
        page_context.params.validation_report = Some(
            create_simple_report(String::from("authenticate_signed_message"), String::from("Error validating moderator signature."))
        );
        return send_authenticate_page_response(StatusCode::BAD_REQUEST, page_context).await;
    }

    return Redirect::to(&context.params.redirect_to).into_response()
}

async fn validate_authenticate_form(form: &PostAuthenticatePageParams) -> Result<(), Report> {
    if let Err(report) = form.validate() {
        return Err(report);
    }
    Ok(())
}

pub async fn send_authenticate_page_response(status: StatusCode, context: AuthenticatePageContext) -> Response {
    (
        status,
        html_to_response(
            &context,
            |hx_target, context| async move {
                match hx_target.as_str() {
                    "main-article" => render_template!(AuthenticateTemplate, &context, page_content),
                    _ => render_template!(AuthenticateTemplate, &context),
                }
            }
        ).await
    ).into_response()
}
