mod api;
mod app;
mod audio;
mod catalog;
mod config;

use tracing::info;

#[tokio::main]
async fn main() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let settings = config::Settings::from_env().expect("invalid configuration");
    catalog::init(&settings.data_dir).expect("failed to initialize sound catalog");
    let store = catalog::new_store(settings.data_dir.clone());
    let app = app::router(store, settings.clone());

    let listener = tokio::net::TcpListener::bind(&settings.bind_address)
        .await
        .unwrap();
    log_startup_banner(&settings.bind_address);
    axum::serve(listener, app)
        .with_graceful_shutdown(interruption())
        .await
        .unwrap();
}

async fn interruption() {
    tokio::signal::ctrl_c().await.unwrap();
}

fn log_startup_banner(bind_address: &str) {
    let port = bind_address.rsplit(':').next().unwrap_or("?");
    info!("listening on {bind_address}");
    info!("local:   curl http://127.0.0.1:{port}/help");
    info!("lan:     curl http://<this machine ip>:{port}/help (find ip: hostname -I)");
}
