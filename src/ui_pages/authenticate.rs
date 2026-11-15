use std::error::Error;
use askama::Template;
use garde::{ Report };

use crate::router::routes::authenticate::AuthenticatePageContext;
use crate::router::validation::report_has_field;
use crate::ui_primitives::alert::AlertTemplate;
use crate::util::crypto;

#[derive(Template)]
#[template(path = "ui_pages/authenticate.html", blocks = ["page_content"])]
pub struct AuthenticateTemplate<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
    authenticate_public_key: String,
    authenticate_signing_message: String,
    redirect_to: String,
    validation_alert: Option<AlertTemplate<'a>>,
}
impl<'a> AuthenticateTemplate<'a> {
    pub async fn new(context: &'a AuthenticatePageContext) -> Result<AuthenticateTemplate<'a>, Box<dyn Error>> {
        let validation_alert = get_validation_alert(&context.params.validation_report);

        let authenticate_signing_message = if context.params.authenticate_public_key.len() > 0 {
            crypto::random_message_to_sign_now_window(&context.params.authenticate_public_key, 900, 900, 0)
        } else {
            String::from("")
        };

        let authenticate_public_key = context.params.authenticate_public_key.clone();

        Ok(AuthenticateTemplate {
            _phantom: std::marker::PhantomData,
            authenticate_public_key,
            authenticate_signing_message,
            redirect_to: context.params.redirect_to.clone(),
            validation_alert,
        })
    }
}

fn get_validation_alert<'a>(report: &Option<Report>) -> Option<AlertTemplate<'a>> {
    match report {
        Some(report) => {
            let mut message_html: String = "".to_owned();

            if report_has_field(report, "server_error") {
                message_html.push_str("<p>A system error occurred. Please notify the site admins if this continues to happen.</p>");
            }
            if report_has_field(report, "authenticate_signed_message") {
                message_html.push_str("<p>You do not have access to this page.</p>");
            }

            Some(AlertTemplate {
                variant: "danger",
                message_html,
            })
        },
        _ => None,
    }
}