use actix_web::dev::Server;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use actix_session::SessionMiddleware;
use actix_session::storage::RedisSessionStore;
use actix_web::cookie::Key;
use actix_web_flash_messages::FlashMessagesFramework;
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_lab::middleware::from_fn;
use secrecy::{ExposeSecret, Secret};
use tracing_actix_web::TracingLogger;
use crate::email_client::EmailClient;
use crate::configuration::Settings;
use sqlx::postgres::PgPoolOptions;
use crate::authentication::reject_anonymous_users;
use crate::configuration::DatabaseSettings;
use crate::routes::{confirm, health_check, subscribes, publish_newsletter, index, login_form, login, admin_dashboard, change_password_form, change_password, logout};

pub struct Application {
    port: u16,
    server: Server
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);

        let sender_email = configuration.email_client
            .sender()
            .expect("Invalid sender email address.");

        let timeout = configuration.email_client.timeout();

        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );

        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri,
        ).await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub struct ApplicationBaseUrl(pub String);

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>
) -> Result<Server, anyhow::Error> {
    let db_pool = Data::new(db_pool);
    let email_client = Data::new(email_client);
    let base_url = Data::new(ApplicationBaseUrl(base_url));
    let message_cookies = CookieMessageStore::builder(
        Key::from(hmac_secret.expose_secret().as_bytes())
    ).build();
    let message_framework = FlashMessagesFramework::builder(message_cookies)
        .build();
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(redis_store.clone(), secret_key.clone()))
            .wrap(TracingLogger::default())
            .service(
                web::scope("/admin")
                    .wrap(from_fn(reject_anonymous_users))
                    .route("/logout", web::post().to(logout))
                    .route("/dashboard", web::get().to(admin_dashboard))
                    .route("/password", web::get().to(change_password_form))
                    .route("/password", web::post().to(change_password))
            )
            .route("/", web::get().to(index))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/health-check", web::get().to(health_check))
            .route("/subscribes", web::post().to(subscribes))
            .route("/subscribes/confirm", web::get().to(confirm))
            .route("/newsletters", web::post().to(publish_newsletter))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(Data::new(HmacSecret(hmac_secret.clone())))
    })
        .listen(listener)?
        .run();

    Ok(server)
}

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);
