use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;
use tracing::Instrument;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String
}

pub async fn subscribes(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>
) -> HttpResponse {
    let request_id = Uuid::new_v4();

    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name
    );

    let _request_span_guard = request_span.enter();

    let query_span = tracing::info_span!(
        "Saving new subscriber details in database"
    );

    match sqlx::query!(r#"
        insert into subscriptions (id, email, name, subscribed_at)
        values ($1, $2, $3, $4)
    "#, Uuid::new_v4(), form.email, form.name, Utc::now())
        .execute(pool.get_ref())
        .instrument(query_span)
        .await
    {
        Ok(_) => {
            HttpResponse::Ok().finish()
        },
        Err(e) => {
            tracing::error!("Failed to execute query: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    };

    HttpResponse::Ok().finish()
}