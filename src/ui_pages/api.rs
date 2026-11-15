use std::error::Error;
use askama::Template;

use crate::router::routes::api::ApiPageContext;

use crate::util::secrets::{ secrets_config };

#[derive(Template)]
#[template(path = "ui_pages/api.html", blocks = ["page_content"])]
pub struct ApiTemplate<'a> {
    ipfs_share_gateway: &'a str,
    website_host: &'a str,
}
impl<'a> ApiTemplate<'a> {
    pub async fn new(_context: &'a ApiPageContext) -> Result<ApiTemplate<'a>, Box<dyn Error>> {

        let secrets = secrets_config();
        let website_host = &secrets.website.hostname;
        let ipfs_share_gateway = &secrets.ipfs.share_gateway;

        Ok(ApiTemplate { ipfs_share_gateway, website_host })
    }
}

