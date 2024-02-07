use zero2prod::configuration::get_configuration;
use zero2prod::startups::build;
use zero2prod::telemetry::{init_subscriber, get_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);

    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to load configs file");

   let server = build(configuration).await?;

    server.await?;

    Ok(())
}
