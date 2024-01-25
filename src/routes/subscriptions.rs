use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;
use unicode_segmentation::UnicodeSegmentation;
use crate::domain::NewSubscriber;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String
}

#[tracing::instrument(
    name = "Adding new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribes(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>
) -> HttpResponse {
    match insert_subscriber(&pool, &form).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in database",
    skip(form, pool)
)]
pub async fn insert_subscriber(pool: &PgPool, new_subscriber: &NewSubscriber) -> Result<(), sqlx::Error> {
    sqlx::query!(r#"
        insert into subscriptions (id, email, name, subscribed_at)
        values ($1, $2, $3, $4)
    "#, Uuid::new_v4(), new_subscriber.email, new_subscriber.name, Utc::now())
        .execute(pool)
        .await
        .map(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;

        Ok(())
}