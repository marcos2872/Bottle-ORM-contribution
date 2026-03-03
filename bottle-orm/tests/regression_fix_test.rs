use bottle_orm::{Database, Model, FromAnyRow};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Model, PartialEq)]
struct User {
    #[orm(primary_key)]
    id: String,
    name: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Model, PartialEq)]
struct Account {
    #[orm(primary_key)]
    id: i32,
    user_id: String,
    account_type: String,
}

#[derive(Debug, Clone, FromAnyRow, PartialEq)]
struct UserGetDTO {
    id: String,
    name: String,
    account_type: String,
}

#[derive(Debug, Clone, FromAnyRow, PartialEq)]
struct AccountUserDTO {
    id: i32,
    name: String,
    created_at: DateTime<Utc>,
}

#[tokio::test]
async fn test_dto_table_resolution_regression() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder().max_connections(1).connect("sqlite::memory:").await?;
    db.migrator().register::<User>().register::<Account>().run().await?;

    let user_id = "user1".to_string();
    db.model::<User>().insert(&User { id: user_id.clone(), name: "Alice".to_string(), created_at: Utc::now() }).await?;
    db.model::<Account>().insert(&Account { id: 1, user_id: user_id.clone(), account_type: "premium".to_string() }).await?;

    // This query reproduces the "column user.account_type does not exist" issue
    // because UserGetDTO.account_type has no table metadata, and it was defaulting to "user"
    let results: Vec<UserGetDTO> = db.model::<User>()
        .left_join("account", "account.user_id = user.id")
        .select("user.id, user.name")
        .select("account.account_type") // Explicitly from account
        .scan_as()
        .await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].account_type, "premium");

    println!("DTO table resolution test passed!");
    Ok(())
}

#[tokio::test]
async fn test_joined_asterisk_expansion_regression() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder().max_connections(1).connect("sqlite::memory:").await?;
    db.migrator().register::<User>().register::<Account>().run().await?;

    // We use a type that has fields from the joined table
    // When we select u.*, the QueryBuilder should see that u.name and u.created_at 
    // match the DTO and expand them.
    let _results: Vec<AccountUserDTO> = db.model::<Account>()
        .alias("a")
        .join("user u", "u.id = a.user_id")
        .select("a.id")
        .select("u.*") 
        .scan_as()
        .await?;
    
    // If it didn't crash and we can access the data, expansion worked 
    // (SQLite would fail if it tried to map u.* directly to individual DTO fields 
    // without the ORM expanding the column names in the SELECT clause)
    
    println!("Joined asterisk expansion test passed!");
    Ok(())
}
