use bottle_orm::{Model, Database, BottleEnum};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, BottleEnum)]
pub enum StatusUser {
    Active,
    Pending,
    Banned,
    Unknown,
}

#[derive(Debug, Model, Serialize, Deserialize, Clone)]
pub struct User {
    #[orm(primary_key, size = 21)]
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: Option<String>,
    #[orm(unique, index)]
    pub email: String,
    #[orm(enum)]
    pub status: StatusUser,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Model, Serialize, Deserialize, Clone)]
pub struct Session {
    #[orm(primary_key, size = 21)]
    pub id: String,
    #[orm(foreign_key = "User::id", size = 21)]
    pub user_id: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[tokio::test]
async fn test_option_null_in_join_regression() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = format!("option_test_{}.db", Utc::now().timestamp_nanos_opt().unwrap());
    let db = Database::connect(&format!("sqlite:{}?mode=rwc", db_path)).await?;
    
    // Register and run migrations
    db.migrator()
        .register::<User>()
        .register::<Session>()
        .run()
        .await?;

    let now = Utc::now();

    // 1. Insert user with NULL avatar (Option::None)
    // This NULL value previously caused index misalignment during Joins
    db.model::<User>()
        .insert(&User {
            id: "user_test_1".to_string(),
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            avatar: None,
            email: "john@example.com".to_string(),
            status: StatusUser::Active,
            created_at: now,
            updated_at: now,
        })
        .await?;

    // 2. Insert a session for this user
    db.model::<Session>()
        .insert(&Session {
            id: "sess_test_1".to_string(),
            user_id: "user_test_1".to_string(),
            token: "valid_token_123".to_string(),
            expires_at: now,
        })
        .await?;

    // 3. Perform Join (Session + User)
    // The column index must advance correctly even for NULL values
    // to ensure subsequent fields (email, status, etc.) are decoded from the right columns.
    let result: (Session, User) = db
        .model::<Session>()
        .join("user", "user.id = session.user_id")
        .equals("session.token", "valid_token_123".to_string())
        .first()
        .await?;

    // Validate results
    assert_eq!(result.0.token, "valid_token_123");
    assert_eq!(result.1.id, "user_test_1");
    assert_eq!(result.1.avatar, None);
    assert_eq!(result.1.first_name, "John");
    assert_eq!(result.1.email, "john@example.com");

    // Cleanup database file
    let _ = tokio::fs::remove_file(&db_path).await;

    Ok(())
}
