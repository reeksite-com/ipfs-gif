use std::error::Error;
use askama::Template;

use crate::router::routes::home::HomePageContext;
use crate::router::{ get_hx_target };
use crate::database::{ self, Gif };
use crate::util::{ secrets::secrets_config };

#[derive(Template)]
#[template(path = "ui_pages/home.html", blocks = ["page_content"])]
pub struct HomeTemplate<'a> {
    ipfs_preview_gateway: &'a str,
    needs_title_update: bool,
    popular_gifs: Vec<Gif>,
}
impl<'a> HomeTemplate<'a> {
    pub async fn new(context: &'a HomePageContext) -> Result<HomeTemplate<'a>, Box<dyn Error>> {
        let secrets = secrets_config();
        let ipfs_preview_gateway = &secrets.ipfs.preview_gateway;
        let needs_title_update = if get_hx_target(&context.route_headers).len() > 0 { true } else { false };

        let popular_gifs = database::get_popular_gifs(0, 60).await?;

        Ok(HomeTemplate { ipfs_preview_gateway, needs_title_update, popular_gifs })
    }
}