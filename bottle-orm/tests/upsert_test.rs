use bottle_orm::{Database, Model};

#[derive(Debug, Clone, Model, PartialEq)]
struct User {
    #[orm(primary_key)]
    id: i32,
    username: String,
    age: i32,
}

#[tokio::test]
async fn test_upsert_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::builder().is_test(true).try_init();
    let db = Database::builder().max_connections(1).connect("sqlite::memory:").await?;

    db.migrator().register::<User>().run().await?;

    let user = User {
        id: 1,
        username: "alice".to_string(),
        age: 20,
    };

    // 1. Initial Insert
    let affected = db.model::<User>().debug().upsert(&user, &["id"], &["username", "age"]).await?;
    assert_eq!(affected, 1);

    let fetched: User = db.model::<User>().debug().first().await?;
    assert_eq!(fetched.username, "alice");
    assert_eq!(fetched.age, 20);

    // 2. Upsert (Update)
    let user_updated = User {
        id: 1,
        username: "alice_updated".to_string(),
        age: 25,
    };

    let affected = db.model::<User>().debug().upsert(&user_updated, &["id"], &["username", "age"]).await?;
    // SQLite returns 1 for update usually, but can vary.
    assert!(affected >= 1);

    let fetched: User = db.model::<User>().debug().first().await?;
    assert_eq!(fetched.username, "alice_updated");
    assert_eq!(fetched.age, 25);

    // 3. Upsert with only subset of columns to update
    let user_subset = User {
        id: 1,
        username: "should_not_change".to_string(),
        age: 30,
    };

    // Only update age
    db.model::<User>().debug().upsert(&user_subset, &["id"], &["age"]).await?;

    let fetched: User = db.model::<User>().debug().first().await?;
    assert_eq!(fetched.username, "alice_updated"); // Remained the same
    assert_eq!(fetched.age, 30); // Updated

    println!("Upsert test passed!");
    Ok(())
}
