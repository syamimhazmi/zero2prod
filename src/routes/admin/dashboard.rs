use actix_web::{HttpResponse, web, http::header::ContentType};
use actix_web::http::header::LOCATION;
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;
use crate::session_state::TypedSession;

fn e500<T>(error: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static
{
    actix_web::error::ErrorInternalServerError(error)
}

pub async fn admin_dashboard(
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)?
    {
        get_username(user_id, &pool).await.map_err(e500)?
    } else {
        return Ok(
            HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish()
        )
    };

    Ok(
        HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(admin_dashboard_html(username))
    )
}

#[tracing::instrument(
    name = "Get user's username",
    skip(pool)
)]
async fn get_username(
    user_id: Uuid,
    pool: &PgPool
) -> Result<String, anyhow::Error> {
    let user = sqlx::query!(
        r#"
            select username from users where user_id = $1
        "#,
        user_id
    )
        .fetch_one(pool)
        .await
        .context("Failed to perform a query to retrieve a username")?;

    Ok(user.username)
}

fn admin_dashboard_html(username: String) -> String {
    format!(
        r#"
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta http-equiv="content-type" content="text/html; charset=utf-8">
                    <title>Admin dashboard</title>
                </head>
                <body>
                    <p>Welcome {username}!</p>
                </body>
            </html>
        "#
    )
}