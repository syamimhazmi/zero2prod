use std::fmt::Formatter;
use actix_session::Session;
use actix_web::http::header::LOCATION;
use actix_web::{HttpResponse, web};
use actix_web::error::InternalError;
use actix_web_flash_messages::FlashMessage;
use secrecy::{Secret};
use sqlx::PgPool;
use crate::authentication::{AuthError, Credentials, validate_credentials};
use crate::routes::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>
}

#[tracing::instrument(
    skip(form, pool, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    session: Session
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password
    };

    tracing::Span::current().record(
        "username",
        &tracing::field::display(&credentials.username)
    );

    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

            session.renew();

            session.insert("user_id", user_id)
                .map_err(|err| login_redirect(LoginError::UnexpectedError(err.into())))?;

            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/admin/dashboard"))
                .finish())
        }
        Err(err) => {
            let error = match err {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(err.into()),
                AuthError::UnexpectedError(_) => {
                    LoginError::UnexpectedError(err.into())
                }
            };

            Err(login_redirect(error))
        }
    }
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error)
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

fn login_redirect(error: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(error.to_string()).send();

    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish();

    InternalError::from_response(error, response)
}