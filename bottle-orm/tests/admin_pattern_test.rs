use bottle_orm::{Database, Model, FromAnyRow, Pagination, pagination::Paginated, BottleEnum};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Models (Similar to user's models)
// ============================================================================

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, BottleEnum)]
pub enum StatusUser {
    Active,
    Pending,
    Banned,
    Unknown,
}

#[derive(Debug, Model, Serialize, Clone, PartialEq)]
pub struct User {
    #[orm(primary_key)]
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: Option<String>,
    pub email: String,
    #[orm(enum)]
    pub status: StatusUser,
    pub role: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Model, Serialize, Deserialize, Clone, PartialEq)]
pub struct Account {
    #[orm(primary_key)]
    pub id: String,
    pub user_id: String,
    pub account_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Model, Clone, PartialEq)]
pub struct Session {
    #[orm(primary_key)]
    pub id: String,
    pub user_id: String,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Model, Serialize, Clone, PartialEq)]
pub struct Role {
    #[orm(primary_key)]
    pub id: String,
    pub name: String,
    pub position: i32,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Model, Serialize, Deserialize, Clone, PartialEq)]
pub struct Permissions {
    #[orm(primary_key)]
    pub id: String,
    pub slug: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Model, Serialize, Clone, PartialEq)]
pub struct RolePermissions {
    #[orm(primary_key)]
    pub permission_id: String,
    #[orm(primary_key)]
    pub role_id: String,
    pub added_at: DateTime<Utc>,
}

// ============================================================================
// DTOs (Exactly as user's DTOs)
// ============================================================================

#[derive(Debug, Serialize, FromAnyRow)]
pub struct UsersGet {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: Option<String>, // Note: User model doesn't have it in my simplified version but DTO has
    pub email: String,
    pub status: String,
    pub account_type: String,
    pub role: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_seen: Option<DateTime<Utc>>,
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_admin_list_users_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder().max_connections(1).connect("sqlite::memory:").await?;
    
    db.migrator()
        .register::<User>()
        .register::<Account>()
        .register::<Session>()
        .run()
        .await?;

    // Seed data
    let now = Utc::now();
    let user = User {
        id: "user1".to_string(),
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        avatar: None,
        email: "john@example.com".to_string(),
        status: StatusUser::Active,
        role: Some("admin".to_string()),
        created_at: now,
        updated_at: now,
    };
    db.model::<User>().insert(&user).await?;

    db.model::<Account>().insert(&Account {
        id: "acc1".to_string(),
        user_id: user.id.clone(),
        account_type: "premium".to_string(),
        created_at: now,
    }).await?;

    db.model::<Session>().insert(&Session {
        id: "sess1".to_string(),
        user_id: user.id.clone(),
        last_seen: now,
    }).await?;

    // Reproduce user's query pattern
    let pag = Pagination::new(0, 10);
    let result: Paginated<UsersGet> = pag
        .paginate_as(
            db.model::<User>()
                .debug()
                .left_join("session", "session.user_id = user.id")
                .left_join("account", "account.user_id = user.id")
                .select("user.*")
                .select("session.last_seen")
                .select("account.account_type"),
        )
        .await?;

    assert_eq!(result.total, 1);
    assert_eq!(result.data[0].first_name, "John");
    assert_eq!(result.data[0].account_type, "premium");
    assert!(result.data[0].last_seen.is_some());

    println!("Admin list_users pattern test passed!");
    Ok(())
}

#[tokio::test]
async fn test_admin_list_roles_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder().max_connections(1).connect("sqlite::memory:").await?;
    
    db.migrator()
        .register::<Role>()
        .register::<Permissions>()
        .register::<RolePermissions>()
        .run()
        .await?;

    let now = Utc::now();
    let role = Role {
        id: "role1".to_string(),
        name: "Admin".to_string(),
        position: 1,
        description: "Admin role".to_string(),
        created_at: now,
    };
    db.model::<Role>().insert(&role).await?;

    let perm = Permissions {
        id: "perm1".to_string(),
        slug: "manage_users".to_string(),
        description: "Can manage users".to_string(),
        created_at: now,
    };
    db.model::<Permissions>().insert(&perm).await?;

    db.model::<RolePermissions>().insert(&RolePermissions {
        permission_id: perm.id.clone(),
        role_id: role.id.clone(),
        added_at: now,
    }).await?;

    // 3. Reproduce scan_as with tuple and wildcard
    let role_ids = vec![role.id.clone()];
    let permissions_flat: Vec<(String, Permissions)> = db
        .model::<RolePermissions>()
        .debug()
        .alias("rp")
        .join("permissions p", "p.id = rp.permission_id")
        .in_list("rp.role_id", role_ids)
        .select("rp.role_id")
        .select("p.*")
        .scan_as::<(String, Permissions)>()
        .await?;

    assert_eq!(permissions_flat.len(), 1);
    assert_eq!(permissions_flat[0].0, role.id);
    assert_eq!(permissions_flat[0].1.slug, "manage_users");

    println!("Admin list_roles pattern test passed!");
    Ok(())
}

#[tokio::test]
async fn test_admin_create_role_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder().max_connections(1).connect("sqlite::memory:").await?;
    db.migrator().register::<Role>().run().await?;

    db.model::<Role>().insert(&Role {
        id: "r1".to_string(),
        name: "User".to_string(),
        position: 10,
        description: "".to_string(),
        created_at: Utc::now(),
    }).await?;

    // Reproduce update_raw with where_raw
    let position = 5;
    let affected = db.model::<Role>()
        .alias("r")
        .where_raw("position >=", position)
        .update_raw("position", "position + ?", 1)
        .await?;

    assert_eq!(affected, 1);
    
    let updated: Role = db.model::<Role>().first().await?;
    assert_eq!(updated.position, 11);

    println!("Admin create_role pattern test passed!");
    Ok(())
}

#[tokio::test]
async fn test_query_builder_comprehensive() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder().max_connections(1).connect("sqlite::memory:").await?;
    db.migrator().register::<User>().run().await?;

    let now = Utc::now();
    for i in 1..=5 {
        db.model::<User>().insert(&User {
            id: format!("u{}", i),
            first_name: format!("User{}", i),
            last_name: "Test".to_string(),
            avatar: None,
            email: format!("u{}@test.com", i),
            status: StatusUser::Active,
            role: None,
            created_at: now,
            updated_at: now,
        }).await?;
    }

    // Test distinct
    let count: i64 = db.model::<User>().select("count(distinct last_name)").scalar().await?;
    assert_eq!(count, 1);

    // Test limit/offset
    let page: Vec<User> = db.model::<User>().limit(2).offset(1).scan().await?;
    assert_eq!(page.len(), 2);

    // Test multiple filters
    let filtered: Vec<User> = db.model::<User>()
        .equals("last_name", "Test".to_string())
        .in_list("id", vec!["u1".to_string(), "u2".to_string()])
        .scan()
        .await?;
    assert_eq!(filtered.len(), 2);

    println!("Comprehensive QueryBuilder test passed!");
    Ok(())
}
