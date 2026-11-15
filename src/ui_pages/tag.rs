use std::error::Error;
use std::io;
use askama::Template;

use crate::database::{ self, Gif };
use crate::router::routes::tag::TagPageContext;
use crate::util::{ secrets::secrets_config };

#[derive(Template)]
#[template(path = "ui_pages/tag.html", blocks = ["page_content"])]
pub struct TagTemplate<'a> {
    ipfs_preview_gateway: &'a str,
    gifs: Vec<Gif>,
    tag_name: String,
}
impl<'a> TagTemplate<'a> {
    pub async fn new(context: &'a TagPageContext) -> Result<TagTemplate<'a>, Box<dyn Error>> {
        let secrets = secrets_config();
        let ipfs_preview_gateway = &secrets.ipfs.preview_gateway;
        let tag_name = context.params.tag_hash.split("-").collect::<Vec<&str>>().join(" ");

        let gifs = database::get_gifs_by_tag(&tag_name, 0, 60).await?;

        if gifs.len() == 0 {
            return Err(Box::new(
                io::Error::new(io::ErrorKind::Other, "No GIFs.")
            ))
        }

        Ok(TagTemplate {
            gifs,
            ipfs_preview_gateway,
            tag_name,
        })
    }
}

