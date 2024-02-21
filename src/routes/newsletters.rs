use std::fmt::Formatter;
use actix_web::{HttpResponse, web, ResponseError, HttpRequest};
use sqlx::PgPool;
use crate::routes::error_chain_fmt;
use actix_web::http::StatusCode;
use anyhow::Context;
use crate::email_client::EmailClient;
use crate::domains::SubscriberEmail;
use secrecy::Secret;
use secrecy::ExposeSecret;
use actix_web::http::header::{HeaderMap, HeaderValue};
use base64::Engine;
use reqwest::header;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);

                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();

                response.headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);

                response
            }
        }
    }
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
name = "Get confirm subscribers",
skip(pool)
)]
async fn get_confirmed_subscriber(pool: &PgPool) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscriber = sqlx::query!(
        r#"select email from subscriptions where status = 'confirmed'"#,
    )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| match SubscriberEmail::parse(row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error))
        })
        .collect();

    Ok(confirmed_subscriber)
}

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool
) -> Result<uuid::Uuid, PublishError> {
    let user_id: Option<_> = sqlx::query!(
            r#"
                SELECT user_id
                FROM users
                WHERE username = $1 AND password = $2
            "#,
            credentials.username,
            credentials.password.expose_secret()
        ).fetch_optional(pool)
        .await
        .context("Failed to perform a query to validate auth credentials.")
        .map_err(PublishError::UnexpectedError)?;

    user_id.map(|row| row.user_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid username or password."))
        .map_err(PublishError::AuthError)
}

#[tracing::instrument(
    name = "Publish a newsletter issues",
    skip(body, pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers())
        .map_err(PublishError::AuthError)?;

    tracing::Span::current().record(
        "username",
        &tracing::field::display(&credentials.username)
    );

    let user_id = validate_credentials(credentials, &pool).await?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscriber(&pool).await?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client.send_email(
                    &subscriber.email,
                    &body.title,
                    &body.content.html,
                    &body.content.text,
                ).await
                    .with_context(|| {
                        format!("Failed to send newsletters issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping confirmed subscriber. \
                    Their stored contact is invalid.
                    "
                )
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers.get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;

    let base64encoded_credentials = header_value.strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD.decode(base64encoded_credentials)
        .context("Failed to base64-decode 'Basic' credentials.")?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');

    let username = credentials
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!("A username must be provided in 'Basic' auth.")
        })?.to_string();

    let password = credentials
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!("A password must be provided in 'Basic' auth.")
        })?.to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}