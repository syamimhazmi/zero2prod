use zero2prod::configuration::get_configuration;
use zero2prod::startups::run;
use zero2prod::telemetry::{init_subscriber, get_subscriber};
use sqlx::PgPool;
use std::net::TcpListener;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);

    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to load configuration file");
    let connection_pool = PgPool::connect(
        &configuration.database.connection_string()
    ).await.expect("Failed to connect to Postgres");

    let address = format!("127.0.0.1:{}", configuration.application_port);

    let listener = TcpListener::bind(address)
        .expect("Failed to bind port");

    run(listener, connection_pool)?.await
}