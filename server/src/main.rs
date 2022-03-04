use affect_api::affect::{
    item_service_server::ItemServiceServer, nonprofit_service_server::NonprofitServiceServer,
    user_service_server::UserServiceServer,
};
use affect_server::{
    change::api::{ChangeClient, ChangeCredentials},
    config::ServerConfig,
    firebase::FirebaseAuth,
    interceptors::authn::AuthnInterceptor,
    seed,
    services::{item::ItemServiceImpl, nonprofit::NonprofitServiceImpl, user::UserServiceImpl},
    tonic::async_interceptor::AsyncInterceptorLayer,
};
use affect_storage::{
    stores::{
        account::PgAccountStore, item::PgItemStore, nonprofit::PgNonprofitStore, user::PgUserStore,
    },
    PgPool,
};
use log::info;
use std::{sync::Arc, time::Duration};
use tonic::transport::Server;
use tower::ServiceBuilder;

fn load_config() -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let config_path = std::env::var("CONFIG_PATH").ok();
    let config = std::env::var("CONFIG").ok();

    let config_str = match (config_path, config) {
        (None, Some(config)) => config,
        (Some(config_path), None) => std::fs::read_to_string(config_path)?,
        (Some(_), Some(_)) => {
            panic!("Only one of CONFIG and CONFIG_PATH environment variables should be specified")
        }
        (None, None) => {
            panic!("Either CONFIG or CONFIG_PATH environment variables should be specified")
        }
    };

    Ok(toml::from_str::<ServerConfig>(&config_str)?)
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    info!("Loading config");
    let config = load_config()?;

    // Database connection and stores:
    info!("Connecting to database");
    let pool = Arc::new(PgPool::connect(config.postgres.uri).await?);
    let user_store = Arc::new(PgUserStore::new(pool.clone()));
    let nonprofit_store = Arc::new(PgNonprofitStore::new(pool.clone()));
    let item_store = Arc::new(PgItemStore::new(pool.clone()));
    let account_store = Arc::new(PgAccountStore::new(pool.clone()));

    info!("Running migrations (if any)");
    pool.run_migrations().await?;

    // Dependencies:
    let firebase_auth =
        Arc::new(FirebaseAuth::load(config.firebase.gwk_url, config.firebase.project_id).await?);
    let change_client = Arc::new(ChangeClient::new(ChangeCredentials::new(
        config.change.public_key,
        config.change.secret_key,
    )));
    let plaid_client = Arc::new(plaid::Client::new(
        config.plaid.client_id,
        config.plaid.secret_key,
        config.plaid.env.parse()?,
    ));

    // Seed database with data.
    seed::insert_nonprofits(nonprofit_store.clone(), change_client).await?;

    // Interceptors/middleware:
    let authn_interceptor_layer = AsyncInterceptorLayer::new(AuthnInterceptor::new(
        firebase_auth.clone(),
        user_store.clone(),
    ));
    let middleware = ServiceBuilder::new()
        .timeout(Duration::from_secs(30))
        .layer(authn_interceptor_layer)
        .into_inner();

    // Services:
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(affect_api::FILE_DESCRIPTOR_SET)
        .build()?;
    let user_service = UserServiceImpl::new(user_store.clone(), firebase_auth.clone());
    let nonprofit_service = NonprofitServiceImpl::new(nonprofit_store.clone());
    let item_service = ItemServiceImpl::new(
        item_store.clone(),
        account_store.clone(),
        plaid_client.clone(),
    );

    let port: u16 = match (config.port, config.port_env_var) {
        (None, Some(port_env_var)) => std::env::var(&port_env_var)?.parse()?,
        (Some(port), None) => port,
        _ => panic!("Expected"),
    };
    let addr = format!("0.0.0.0:{0}", port).parse()?;
    info!("Starting server: {:?}", addr);
    Server::builder()
        .accept_http1(true)
        .layer(middleware)
        .add_service(reflection_service)
        .add_service(tonic_web::enable(UserServiceServer::new(user_service)))
        .add_service(tonic_web::enable(NonprofitServiceServer::new(
            nonprofit_service,
        )))
        .add_service(tonic_web::enable(ItemServiceServer::new(item_service)))
        .serve(addr)
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = run().await;
    if result.is_err() {
        panic!("Failed to run: {:?}", result);
    }
    Ok(())
}
