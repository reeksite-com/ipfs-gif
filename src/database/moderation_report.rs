use std::collections::HashMap;
use std::error::Error;
use chrono::NaiveDateTime;
use sqlx::{
    Row,
    FromRow,
    MySql,
    Type,
};
use serde::Serialize;
use strum_macros::{ Display, EnumString };

use super::get_pool;
use super::gifs::{ Gif, GifModerationStatus };

#[derive(Clone, Debug, Default, Display, EnumString, PartialEq, Serialize, Type)]
#[sqlx(type_name = "report_type")]
#[sqlx(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ModerationReportType {
    Dcma,
    Illegal,
    Doxxing,
    Gore,
    Sexual,
    #[default]
    Other,
}

#[derive(Clone, Debug, Default, Display, EnumString, PartialEq, Serialize, Type)]
#[sqlx(type_name = "moderator_review")]
#[sqlx(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ModeratorReviewStatus {
    Ignored,
    Accepted,
    #[default]
    None,
}

#[allow(unused)]
#[derive(Clone, Debug, Default, FromRow)]
pub struct ModerationReport {
    pub id: u64,
    pub gif_id: u64,
    pub report_time: NaiveDateTime,
    pub report_type: ModerationReportType,
    pub copyright_holder_name: String,
    pub reporter_public_key: String,
    pub reporter_ip_address: String,
    pub reporter_name: String,
    pub reporter_mailing_address: String,
    pub reporter_phone: String,
    pub reporter_email: String,
    pub reporter_attestation: String,
    pub moderator_review: ModeratorReviewStatus,
}

#[allow(unused)]
#[derive(Clone, Debug, Default, FromRow)]
pub struct ModerationCounterClaim {
    pub id: u64,
    pub report_id: u64,
    pub counter_claim_time: NaiveDateTime,
    pub counter_claimant_public_key: String,
    pub counter_claimant_ip_address: String,
    pub counter_claimant_name: String,
    pub counter_claimant_mailing_address: String,
    pub counter_claimant_phone: String,
    pub counter_claimant_email: String,
    pub counter_claimant_attestation: String,
}

pub async fn get_moderation_reports_by_gif_id(
    gif_id: u64
) -> Result<Vec<ModerationReport>, Box<dyn Error>> {
    Ok(
        sqlx::query_as::<MySql, ModerationReport>(r#"
            SELECT moderation_reports.*
            FROM moderation_reports
            WHERE moderation_reports.gif_id=?
            LIMIT 10
        "#)
            .bind(gif_id)
            .fetch_all(get_pool())
            .await?
    )
}

#[derive(Clone, Debug, Default)]
pub struct ModerationReportWithCounterClaims {
    pub report: ModerationReport,
    pub gif: Gif,
    pub counter_claims: Vec<ModerationCounterClaim>,
}

#[derive(Clone, Debug, Default)]
pub struct ModerationReporterSummary {
    pub reporter_public_key: String,
    pub reporter_ip_address: String,
    pub reports: Vec<ModerationReportWithCounterClaims>,
}

pub async fn get_moderation_report_queue() -> Result<Vec<ModerationReporterSummary>, Box<dyn Error>> {
    let rows = sqlx::query(r#"
        SELECT
            r.id AS report_id,
            r.gif_id AS gif_id,
            r.report_time AS report_time,
            r.report_type AS report_type,
            r.copyright_holder_name AS copyright_holder_name,
            r.reporter_public_key AS reporter_public_key,
            r.reporter_ip_address AS reporter_ip_address,
            r.reporter_name AS reporter_name,
            r.reporter_email AS reporter_email,

            c.id AS counter_claim_id,
            c.report_id AS counter_claim_report_id,
            c.counter_claim_time AS counter_claim_time,
            c.counter_claimant_public_key AS counter_claimant_public_key,
            c.counter_claimant_ip_address AS counter_claimant_ip_address,
            c.counter_claimant_name AS counter_claimant_name,
            c.counter_claimant_email AS counter_claimant_email,

            g.cid AS gif_cid,
            g.filename AS gif_filename,
            g.width AS gif_width,
            g.height AS gif_height
        FROM moderation_reports r
        JOIN gifs g
            ON g.id = r.gif_id
        LEFT JOIN moderation_counter_claims c
            ON c.report_id = r.id
        WHERE (r.reporter_public_key, r.reporter_ip_address) IN (
            SELECT rr.reporter_public_key, rr.reporter_ip_address
            FROM (
                SELECT DISTINCT
                    reporter_public_key,
                    reporter_ip_address
                FROM moderation_reports
                ORDER BY reporter_public_key
                LIMIT 10
            ) rr
        ) AND r.moderator_review = 'none'
        ORDER BY r.reporter_public_key, r.reporter_ip_address, r.id, c.counter_claim_time
    "#)
        .fetch_all(get_pool())
        .await?;

    let mut reports: HashMap<(String, String), Vec<ModerationReportWithCounterClaims>> = HashMap::new();
    let mut report_buckets: HashMap<u64, ModerationReportWithCounterClaims> = HashMap::new();
    let mut reporter_meta: HashMap<u64, (String, String)> = HashMap::new();

    for row in rows {
        let report_id: u64 = row.try_get("report_id")?;
        let gif_id: u64 = row.try_get("gif_id")?;
        let report_time: NaiveDateTime = row.try_get("report_time")?;
        let report_type: ModerationReportType = row.try_get("report_type")?;
        let copyright_holder_name: String = row.try_get("copyright_holder_name")?;
        let reporter_public_key: String = row.try_get("reporter_public_key")?;
        let reporter_ip_address: String = row.try_get("reporter_ip_address")?;
        let reporter_name: String = row.try_get("reporter_name")?;
        let reporter_email: String = row.try_get("reporter_email")?;

        let gif_cid: Option<String> = row.try_get("gif_cid")?;
        let gif_filename: String = row.try_get("gif_filename")?;
        let gif_width: u32 = row.try_get("gif_width")?;
        let gif_height: u32 = row.try_get("gif_height")?;

        if !report_buckets.contains_key(&report_id) {
            let report = ModerationReportWithCounterClaims{
                report: ModerationReport {
                    id: report_id,
                    gif_id,
                    report_time,
                    report_type,
                    copyright_holder_name,
                    reporter_public_key: reporter_public_key.clone(),
                    reporter_ip_address: reporter_ip_address.clone(),
                    reporter_name: reporter_name.clone(),
                    reporter_email: reporter_email.clone(),
                    ..ModerationReport::default()
                },
                gif: Gif {
                    id: gif_id,
                    cid: gif_cid,
                    filename: gif_filename,
                    width: gif_width,
                    height: gif_height,
                    ..Gif::default()
                },
                counter_claims: Vec::new(),
            };

            report_buckets.insert(report_id, report);
            reporter_meta.insert(report_id, (reporter_public_key.clone(), reporter_ip_address.clone()));
        }

        let counter_claim_id: Option<u64> = row.try_get("counter_claim_id")?;
        if let Some(counter_claim_id) = counter_claim_id {
            // Also fetch remaining counter-claim fields.
            let counter_claim_time: NaiveDateTime = row.try_get("counter_claim_time")?;
            let counter_claimant_public_key: String =
                row.try_get("counter_claimant_public_key")?;
            let counter_claimant_ip_address: String =
                row.try_get("counter_claimant_ip_address")?;
            let counter_claimant_name: String =
                row.try_get("counter_claimant_name")?;
            let counter_claimant_email: String =
                row.try_get("counter_claimant_email")?;

            // report_id in c should match, but we rely on report_id from the row anyway
            let counter_claim = ModerationCounterClaim {
                id: counter_claim_id,
                report_id: row.try_get("counter_claim_report_id")?,
                counter_claim_time,
                counter_claimant_public_key,
                counter_claimant_ip_address,
                counter_claimant_name,
                counter_claimant_email,
                ..ModerationCounterClaim::default()
            };

            // Append to the bucket.
            if let Some(report) = report_buckets.get_mut(&report_id) {
                report.counter_claims.push(counter_claim);
            }
        }
    }

    for (report_id, report) in report_buckets {
        if let Some((pub_key, ip)) = reporter_meta.get(&report_id) {
            reports
                .entry((pub_key.clone(), ip.clone()))
                .or_default()
                .push(report);
        }
    }

    let mut out = Vec::with_capacity(reports.len());
    for ((reporter_public_key, reporter_ip_address), reports) in reports {
        out.push(ModerationReporterSummary {
            reporter_public_key,
            reporter_ip_address,
            reports,
        });
    }

    Ok(out)
}

pub async fn create_moderation_report(
    moderation_report: ModerationReport
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    let pool = get_pool();

    let bad_actor_count: i64 = sqlx::query_scalar(r#"
        SELECT COUNT(*)
        FROM moderation_bad_actors
        WHERE (
            public_key=?
            OR (public_key='' AND ip_address=?)
        ) AND ban_time >= NOW() - INTERVAL 30 DAY;
    "#)
        .bind(&moderation_report.reporter_public_key)
        .bind(&moderation_report.reporter_ip_address)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| 0);
    if bad_actor_count > 0 {
        return Ok(0);
    }

    let gif_id = moderation_report.gif_id;

    let existing_report_count: i64 = sqlx::query_scalar(r#"
        SELECT COUNT(DISTINCT reporter_ip_address)
        FROM moderation_reports
        WHERE report_type=?
    "#)
        .bind(&moderation_report.report_type)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| 0);

    let gif_moderation_status = match &moderation_report.report_type {
        ModerationReportType::Dcma => Some(GifModerationStatus::DcmaTakedownNotice),
        ModerationReportType::Illegal => if existing_report_count >= 50 { Some(GifModerationStatus::IllegalRemoved) } else { None },
        ModerationReportType::Doxxing => if existing_report_count >= 50 { Some(GifModerationStatus::DoxxingRemoved) } else { None },
        ModerationReportType::Gore => if existing_report_count >= 50 { Some(GifModerationStatus::GoreRemoved) } else { None },
        ModerationReportType::Sexual => if existing_report_count >= 50 { Some(GifModerationStatus::SexualRemoved) } else { None },
        ModerationReportType::Other => None,
    };

    let result = sqlx::query(r#"
        INSERT INTO moderation_reports (
            gif_id, report_time, report_type, copyright_holder_name, reporter_public_key, reporter_ip_address,
            reporter_name, reporter_mailing_address, reporter_phone, reporter_email, reporter_attestation
        )
        VALUES (?, NOW(), ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#)
        .bind(moderation_report.gif_id)
        .bind(moderation_report.report_type)
        .bind(moderation_report.copyright_holder_name)
        .bind(moderation_report.reporter_public_key)
        .bind(moderation_report.reporter_ip_address)
        .bind(moderation_report.reporter_name)
        .bind(moderation_report.reporter_mailing_address)
        .bind(moderation_report.reporter_phone)
        .bind(moderation_report.reporter_email)
        .bind(moderation_report.reporter_attestation)
        .execute(pool)
        .await;
    
    let moderation_report_id = match result {
        Ok(result) => u64::try_from(result.last_insert_id()).unwrap_or_else(|_| 999),
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    if let Some(gif_moderation_status) = gif_moderation_status {
        let result = sqlx::query(r#"
            UPDATE gifs
            SET moderation_status=?
            WHERE gifs.id=?
        "#)
            .bind(gif_moderation_status)
            .bind(gif_id)
            .execute(pool)
            .await;
        
        if let Err(e) = result {
            return Err(Box::new(e));
        }
    }

    Ok(moderation_report_id)
}

pub async fn create_moderation_counter_claim(
    moderation_counter_claim: ModerationCounterClaim
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    let pool = get_pool();

    let bad_actor_count: i64 = sqlx::query_scalar(r#"
        SELECT COUNT(*)
        FROM moderation_bad_actors
        WHERE (
            public_key=?
            OR (public_key='' AND ip_address=?)
        ) AND ban_time >= NOW() - INTERVAL 30 DAY;
    "#)
        .bind(&moderation_counter_claim.counter_claimant_public_key)
        .bind(&moderation_counter_claim.counter_claimant_ip_address)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| 0);
    if bad_actor_count > 0 {
        return Ok(0);
    }

    let moderation_report = match sqlx::query_as::<MySql, ModerationReport>(r#"
        SELECT moderation_reports.*
        FROM moderation_reports
        WHERE moderation_reports.id=?
        LIMIT 1
    "#)
        .bind(moderation_counter_claim.report_id)
        .fetch_one(get_pool())
        .await
    {
        Ok(moderation_report) => moderation_report,
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    let gif_moderation_status = match &moderation_report.report_type {
        ModerationReportType::Dcma => Some(GifModerationStatus::DcmaCounterClaim),
        _ => None,
    };

    let result = sqlx::query(r#"
        INSERT INTO moderation_counter_claims (
            report_id, counter_claim_time, counter_claimant_public_key, counter_claimant_ip_address,
            counter_claimant_name, counter_claimant_mailing_address, counter_claimant_phone, counter_claimant_email, counter_claimant_attestation
        )
        VALUES (?, NOW(), ?, ?, ?, ?, ?, ?, ?)
    "#)
        .bind(moderation_counter_claim.report_id)
        .bind(moderation_counter_claim.counter_claimant_public_key)
        .bind(moderation_counter_claim.counter_claimant_ip_address)
        .bind(moderation_counter_claim.counter_claimant_name)
        .bind(moderation_counter_claim.counter_claimant_mailing_address)
        .bind(moderation_counter_claim.counter_claimant_phone)
        .bind(moderation_counter_claim.counter_claimant_email)
        .bind(moderation_counter_claim.counter_claimant_attestation)
        .execute(pool)
        .await;
    
    let moderation_counter_claim_id = match result {
        Ok(result) => u64::try_from(result.last_insert_id()).unwrap_or_else(|_| 999),
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    if let Some(gif_moderation_status) = gif_moderation_status {
        let result = sqlx::query(r#"
            UPDATE gifs
            SET moderation_status = ?
            WHERE gifs.id = ?
        "#)
            .bind(gif_moderation_status)
            .bind(moderation_report.gif_id)
            .execute(pool)
            .await;
        
        if let Err(e) = result {
            return Err(Box::new(e));
        }
    }

    Ok(moderation_counter_claim_id)
}


pub async fn accept_moderation_report(
    report_id: u64
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let pool = get_pool();

    let report = sqlx::query_as::<MySql, ModerationReport>(r#"
        SELECT moderation_reports.*
        FROM moderation_reports
        WHERE moderation_reports.id=?
        LIMIT 1
    "#)
        .bind(report_id)
        .fetch_one(pool)
        .await?;
    
    let gif = sqlx::query_as::<MySql, Gif>(r#"
        SELECT gifs.*
        FROM gifs
        WHERE gifs.id=?
        LIMIT 1
    "#)
        .bind(report.gif_id)
        .fetch_one(pool)
        .await?;

    if gif.moderation_status == GifModerationStatus::None {
        let new_moderation_status = match report.report_type {
            ModerationReportType::Dcma => gif.moderation_status,
            ModerationReportType::Illegal => GifModerationStatus::IllegalRemoved,
            ModerationReportType::Doxxing => GifModerationStatus::DoxxingRemoved,
            ModerationReportType::Gore => GifModerationStatus::GoreRemoved,
            ModerationReportType::Sexual => GifModerationStatus::SexualRemoved,
            ModerationReportType::Other => GifModerationStatus::OtherRemoved,
        };

        let _ = sqlx::query_as::<MySql, Gif>(r#"
            UPDATE gifs
            SET moderation_status=?
            WHERE id=?
            LIMIT 1
        "#)
            .bind(new_moderation_status)
            .bind(gif.id)
            .fetch_optional(pool)
            .await?;
    }

    let _ = sqlx::query_as::<MySql, Gif>(r#"
        UPDATE moderation_reports
        SET moderator_review=?
        WHERE id=?
        LIMIT 1
    "#)
        .bind(
            if report.report_type == ModerationReportType::Dcma {
                ModeratorReviewStatus::Ignored
            } else {
                ModeratorReviewStatus::Accepted
            }
        )
        .bind(report_id)
        .fetch_optional(pool)
        .await?;

    Ok(())
}


pub async fn reject_moderation_report(
    report_id: u64
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let pool = get_pool();

    let report = sqlx::query_as::<MySql, ModerationReport>(r#"
        SELECT moderation_reports.*
        FROM moderation_reports
        WHERE moderation_reports.id=?
        LIMIT 1
    "#)
        .bind(report_id)
        .fetch_one(pool)
        .await?;
    
    let gif = sqlx::query_as::<MySql, Gif>(r#"
        SELECT gifs.*
        FROM gifs
        WHERE gifs.id=?
        LIMIT 1
    "#)
        .bind(report.gif_id)
        .fetch_one(pool)
        .await?;

    let _ = sqlx::query_as::<MySql, Gif>(r#"
        UPDATE gifs
        SET moderation_status=?
        WHERE id=?
        LIMIT 1
    "#)
        .bind(GifModerationStatus::ManuallyReviewed)
        .bind(gif.id)
        .fetch_optional(pool)
        .await?;

    let _ = sqlx::query_as::<MySql, Gif>(r#"
        DELETE FROM moderation_reports
        WHERE id=?
        LIMIT 1
    "#)
        .bind(report_id)
        .fetch_optional(pool)
        .await?;

    Ok(())
}

pub async fn reject_moderation_reporter(
    public_key: &str,
    ip_address: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let pool = get_pool();

    let _ = sqlx::query_as::<MySql, ModerationReport>(r#"
        DELETE FROM moderation_reports
        WHERE reporter_public_key=? AND reporter_ip_address=?
        LIMIT 1
    "#)
        .bind(public_key)
        .bind(ip_address)
        .fetch_optional(pool)
        .await?;
    
    let _ = sqlx::query(r#"
        INSERT INTO moderation_bad_actors (public_key, ip_address)
        VALUES (?, ?)
        ON DUPLICATE KEY UPDATE
        ban_time = NOW();
    "#)
        .bind(public_key)
        .bind(ip_address)
        .execute(pool)
        .await?;
    
    Ok(())
}