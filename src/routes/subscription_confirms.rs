use actix_web::{HttpResponse, web};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Confirm a pending subscriber"
    skip(parameters, pool)
)]
pub async fn confirm(parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> HttpResponse {
    let id = match get_subscriber_id_from_token(
        &pool, &parameters.subscription_token
    ).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    match id {
        None => HttpResponse::Unauthorized().finish(),
        Some(subscriber_id) => {
            if confirm_subscriber(&pool, subscriber_id).await.is_err() {
                return HttpResponse::InternalServerError().finish()
            }

            HttpResponse::Ok().finish()
        }
    }
}

#[tracing::instrument(
    name = "Mark subscriber as confirmed",
    skip(subscriber_id, pool)
)]
pub async fn confirm_subscriber(
    pool: &PgPool,
    subscriber_id: Uuid
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"update subscriptions set status = 'confirmed' where id = $1"#,
        subscriber_id
    )
        .execute(pool)
        .await
        .map(|errors| {
            tracing::error!("Failed to execute query {:?}", errors);

            errors
        })?;

    Ok(())
}

#[tracing::instrument(
    name = "Get subscriber_id from token"
    skip(subscription_token, pool)
)]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "select subscriber_id from subscription_tokens \
            where subscription_token = $1
        ",
        subscription_token
    ).fetch_optional(pool)
        .await
        .map_err(|errors| {
            tracing::error!("Failed to execute query, {:?}", errors);

            errors
        })?;

    Ok(result.map(|record| record.subscriber_id))
}