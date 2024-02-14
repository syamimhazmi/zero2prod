use std::fmt::Formatter;
use actix_web::{HttpResponse, web, ResponseError};
use sqlx::PgPool;
use crate::routes::error_chain_fmt;
use actix_web::http::StatusCode;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error)
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

struct ConfirmedSubscriber {
    email: String,
}

#[tracing::instrument(
    name = "Get confirm subscribers",
    skip(pool)
)]
async fn get_confirmed_subscriber(pool: &PgPool) -> Result<Vec<ConfirmedSubscriber>, anyhow::Error> {
    let rows = sqlx::query_as!(
        ConfirmedSubscriber,
        r#"select email from subscriptions where status = 'confirmed'"#,
    )
        .fetch_all(pool)
        .await?;

    Ok(rows)
}


#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

pub async fn publish_newsletter(
    _body: web::Json<BodyData>,
    pool: web::Data<PgPool>
) -> Result<HttpResponse, PublishError> {
    let _subscribers = get_confirmed_subscriber(&pool).await?;

    Ok(HttpResponse::Ok().finish())
}