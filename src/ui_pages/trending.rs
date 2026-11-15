use std::error::Error;
use askama::Template;

use crate::router::routes::trending::TrendingPageContext;
use crate::router::{ get_hx_target };
use crate::database::{ self, Gif };
use crate::util::{ secrets::secrets_config };

#[derive(Template)]
#[template(path = "ui_pages/trending.html", blocks = ["page_content"])]
pub struct TrendingTemplate<'a> {
    gifs: Vec<Gif>,
    ipfs_preview_gateway: &'a str,
    needs_title_update: bool,
}
impl<'a> TrendingTemplate<'a> {
    pub async fn new(context: &'a TrendingPageContext) -> Result<TrendingTemplate<'a>, Box<dyn Error>> {
        let secrets = secrets_config();
        let ipfs_preview_gateway = &secrets.ipfs.preview_gateway;
        let needs_title_update = if get_hx_target(&context.route_headers).len() > 0 { true } else { false };

        let gifs = database::get_popular_gifs(0, 60).await?;

        Ok(TrendingTemplate {
            gifs,
            ipfs_preview_gateway,
            needs_title_update,
        })
    }
}
