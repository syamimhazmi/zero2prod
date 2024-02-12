use std::error::Error;
use std::fmt::Formatter;
use actix_web::{web, HttpResponse, ResponseError};
use sqlx::{PgPool, Postgres, Transaction};
use chrono::Utc;
use uuid::Uuid;
use crate::domains::{NewSubscriber, SubscriberName, SubscriberEmail};
use crate::email_client::EmailClient;
use crate::startups::ApplicationBaseUrl;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

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
) -> Result<HttpResponse, actix_web::Error> {
    let new_subscriber = match form.0.try_into() {
        Ok(form) => form,
        Err(_) => return Ok(HttpResponse::BadRequest().finish())
    };

    let mut transactions = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return Ok(HttpResponse::InternalServerError().finish())
    };

    let subscriber_id = match insert_subscriber(&mut transactions, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return Ok(HttpResponse::InternalServerError().finish())
    };

    let subscription_token = generate_subscription_token();

    store_token(&mut transactions, subscriber_id, &subscription_token).await?;

    if transactions.commit().await.is_err() {
        return Ok(HttpResponse::InternalServerError().finish());
    }

    let confirmation_email = send_confirmation_email(
        &email_client, new_subscriber, &base_url.0, &subscription_token
    ).await;

    if confirmation_email.is_err() {
        return Ok(HttpResponse::InternalServerError().finish())
    }

    Ok(HttpResponse::Ok().finish())
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();

    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(
    name = "Store subscription token in database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"insert into subscription_tokens (subscription_token, subscriber_id) values ($1, $2)"#,
        subscription_token,
        subscriber_id,
    )
        .execute(transaction)
        .await
        .map_err(|errors| {
            tracing::error!("Failed to execute query: {:?}", errors);

            StoreTokenError(errors)
        })?;

    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber"
    skip(email_client, new_subscriber, base_url)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscribes/confirm?subscription_token={}",
        base_url,
        subscription_token
    );
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
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();

    sqlx::query!(r#"insert into subscriptions (id, email, name, subscribed_at, status)
            values ($1, $2, $3, $4, 'pending_confirmation')"#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
        .execute(transaction)
        .await
        .map(|_| HttpResponse::InternalServerError().finish())?;

    Ok(subscriber_id)
}

pub struct StoreTokenError(sqlx::Error);

impl ResponseError for StoreTokenError {}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            trying to store a subscription token
            "
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

fn error_chain_fmt(
    error: &impl std::error::Error,
    format: &mut Formatter<'_>
) -> std::fmt::Result {
    writeln!(format, "{}\n", error)?;

    let mut current = error.source();

    while let Some(cause) = current {
        writeln!(format, "Caused by:\n\t{}", cause)?;
        current = cause.source()
    }

    Ok(())
}