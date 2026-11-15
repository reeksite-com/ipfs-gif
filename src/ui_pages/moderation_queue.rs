use std::error::Error;
use askama::Template;

use crate::database::{ self, ModerationReporterSummary, ModerationReportType };
use crate::router::routes::moderation_queue::ModerationQueuePageContext;
use crate::util::{ secrets::secrets_config };

#[derive(Template)]
#[template(path = "ui_pages/moderation_queue.html", blocks = ["page_content"])]
pub struct ModerationQueueTemplate<'a> {
    ipfs_preview_gateway: &'a str,
    reporter_summaries: Vec<ModerationReporterSummary>,
}
impl<'a> ModerationQueueTemplate<'a> {
    pub async fn new(_context: &'a ModerationQueuePageContext) -> Result<ModerationQueueTemplate<'a>, Box<dyn Error>> {
        let secrets = secrets_config();
        let ipfs_preview_gateway = &secrets.ipfs.preview_gateway;
        let reporter_summaries = database::get_moderation_report_queue().await?;

        Ok(ModerationQueueTemplate {
            ipfs_preview_gateway,
            reporter_summaries,
        })
    }
}

