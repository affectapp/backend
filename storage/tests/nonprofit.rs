use affect_storage::{database::client::DatabaseClient, stores::nonprofit::*};
use chrono::{TimeZone, Utc};
use testcontainers::clients::Cli;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn create_nonprofit() -> Result<(), anyhow::Error> {
    let docker = Cli::default();
    let container = common::setup_pg_container(&docker).await?;
    let store = container.pool.on_demand();

    let mut expected_nonprofit = NonprofitRow {
        nonprofit_id: Uuid::new_v4(),
        create_time: Utc.timestamp(500, 0),
        update_time: Utc.timestamp(1000, 0),
        change_nonprofit_id: "test_nonprofit_id".to_string(),
        icon_url: "test_icon_url".to_string(),
        name: "name".to_string(),
        ein: "ein".to_string(),
        mission: "mission".to_string(),
        category: "category".to_string(),
    };

    // Insert nonprofit.
    let nonprofit = store
        .add_nonprofit(NewNonprofitRow {
            create_time: expected_nonprofit.create_time.clone(),
            update_time: expected_nonprofit.update_time.clone(),
            change_nonprofit_id: expected_nonprofit.change_nonprofit_id.clone(),
            icon_url: expected_nonprofit.icon_url.clone(),
            name: expected_nonprofit.name.clone(),
            ein: expected_nonprofit.ein.clone(),
            mission: expected_nonprofit.mission.clone(),
            category: expected_nonprofit.category.clone(),
        })
        .await?;

    // Id is generated on insert, don't test that.
    expected_nonprofit.nonprofit_id = nonprofit.nonprofit_id;

    assert_eq!(nonprofit, expected_nonprofit);
    Ok(())
}
