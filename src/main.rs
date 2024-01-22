use secrecy::ExposeSecret;
use zero2prod::configuration::get_configuration;
use zero2prod::startups::run;
use zero2prod::telemetry::{init_subscriber, get_subscriber};
// use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);

    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to load configuration file");
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(
            &configuration.database.connection_string().expose_secret()
        )
        .expect("Failed to connect to Postgres");

    let address = format!("{}:{}", configuration.application.host, configuration.application.port);

    let listener = TcpListener::bind(address)?;

    run(listener, connection_pool)?.await?;

    Ok(())
}