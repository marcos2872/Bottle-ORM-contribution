use bottle_orm::{Database, Model, Op};

#[derive(Debug, Clone, Model, PartialEq)]
struct User {
    #[orm(primary_key)]
    id: i32,
    username: String,
    age: i32,
}

#[tokio::test]
async fn test_update_raw() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder().max_connections(1).connect("sqlite::memory:").await?;

    db.migrator().register::<User>().run().await?;

    let id = 1;
    let user = User {
        id,
        username: "alice".to_string(),
        age: 20,
    };

    db.model::<User>().insert(&user).await?;

    // Increment age using update_raw
    db.model::<User>()
        .filter("id", Op::Eq, id)
        .update_raw("age", "age + 1", 0)
        .await?;

    let updated_user: User = db.model::<User>()
        .filter("id", Op::Eq, id)
        .first()
        .await?;

    assert_eq!(updated_user.age, 21);

    // Update with placeholder
    db.model::<User>()
        .filter("id", Op::Eq, id)
        .update_raw("age", "age + ?", 10)
        .await?;

    let updated_user2: User = db.model::<User>()
        .filter("id", Op::Eq, id)
        .first()
        .await?;

    assert_eq!(updated_user2.age, 31);

    println!("update_raw basic tests passed!");

    // Test soft delete interaction
    #[derive(Debug, Clone, Model, PartialEq)]
    struct SoftUser {
        #[orm(primary_key)]
        id: i32,
        #[orm(soft_delete)]
        deleted_at: Option<chrono::DateTime<chrono::Utc>>,
        age: i32,
    }

    db.migrator().register::<SoftUser>().run().await?;

    let soft_id = 1;
    let soft_user = SoftUser {
        id: soft_id,
        deleted_at: Some(chrono::Utc::now()), // Mark as deleted
        age: 20,
    };
    db.model::<SoftUser>().insert(&soft_user).await?;

    // Try to update deleted user - should affect 0 rows
    let affected = db.model::<SoftUser>()
        .filter("id", Op::Eq, soft_id)
        .update_raw("age", "age + 1", 0)
        .await?;
    
    assert_eq!(affected, 0);

    // Try with_deleted - should affect 1 row
    let affected_with_deleted = db.model::<SoftUser>()
        .with_deleted()
        .filter("id", Op::Eq, soft_id)
        .update_raw("age", "age + 1", 0)
        .await?;
    
    assert_eq!(affected_with_deleted, 1);

    println!("update_raw soft delete test passed!");
    Ok(())
}
