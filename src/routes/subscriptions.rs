use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;
use crate::domains::{NewSubscriber, SubscriberName, SubscriberEmail};
use crate::email_client::EmailClient;
use crate::startups::ApplicationBaseUrl;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String
}

pub fn parse_subscriber(form: FormData) -> Result<NewSubscriber, String> {
    let name = SubscriberName::parse(form.name)?;
    let email = SubscriberEmail::parse(form.email)?;

    Ok(NewSubscriber{ name, email })
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;

        Ok(Self{ name, email })
    }
}

#[tracing::instrument(
    name = "Adding new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribes(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>
) -> HttpResponse {
    let new_subscriber = match form.0.try_into() {
        Ok(form) => form,
        Err(_) => return HttpResponse::BadRequest().finish()
    };

    let insert_subscriber = insert_subscriber(&pool, &new_subscriber).await;

    let confirmation_email = send_confirmation_email(
        &email_client, new_subscriber, &base_url.0
    ).await;

    if insert_subscriber.is_err() || confirmation_email.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber"
    skip(email_client, new_subscriber, base_url)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!("{}/subscribes/confirms", base_url);
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );

    email_client.send_email(
        new_subscriber.email,
        "Welcome to Syamim Hazmi",
        &html_body,
        &plain_body,
    ).await
}

#[tracing::instrument(
    name = "Saving new subscriber details in database",
    skip(new_subscriber, pool)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber
) -> Result<(), sqlx::Error> {
    sqlx::query!(r#"insert into subscriptions (id, email, name, subscribed_at, status)
            values ($1, $2, $3, $4, 'pending_confirmation')"#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
        .execute(pool)
        .await
        .map(|_| HttpResponse::InternalServerError().finish())?;

        Ok(())
}