
use bottle_orm::{Database, Model};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Model, Debug, Clone, Serialize, Deserialize, PartialEq)]
struct NullableUser {
    #[orm(primary_key)]
    id: Uuid,
    username: String,
    role: Option<String>,
}

#[tokio::test]
async fn test_update_to_null() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder().max_connections(1).connect("sqlite::memory:").await?;
    db.sync_table::<NullableUser>().await?;

    let user_id = Uuid::new_v4();
    let user = NullableUser {
        id: user_id,
        username: "test_user".to_string(),
        role: Some("admin".to_string()),
    };

    // Insert user with role
    db.model::<NullableUser>().insert(&user).await?;

    // Verify role is set
    let saved_user: NullableUser = db.model::<NullableUser>()
        .equals("id", user_id.to_string())
        .first()
        .await?;
    assert_eq!(saved_user.role, Some("admin".to_string()));

    // Update role to None (NULL)
    db.model::<NullableUser>()
        .equals("id", user_id.to_string())
        .update("role", None::<String>)
        .await?;

    // Verify role is now NULL
    let updated_user: NullableUser = db.model::<NullableUser>()
        .equals("id", user_id.to_string())
        .first()
        .await?;
    assert_eq!(updated_user.role, None);

    // Update role back to Some
    db.model::<NullableUser>()
        .equals("id", user_id.to_string())
        .update("role", Some("editor".to_string()))
        .await?;

    // Verify role is updated
    let updated_user2: NullableUser = db.model::<NullableUser>()
        .equals("id", user_id.to_string())
        .first()
        .await?;
    assert_eq!(updated_user2.role, Some("editor".to_string()));

    // Test updating via model updates()
    let mut user_to_null = updated_user2.clone();
    user_to_null.role = None;
    db.model::<NullableUser>()
        .equals("id", user_id.to_string())
        .updates(&user_to_null)
        .await?;

    // Verify role is NULL again
    let final_user: NullableUser = db.model::<NullableUser>()
        .equals("id", user_id.to_string())
        .first()
        .await?;
    assert_eq!(final_user.role, None);

    // Test insert with None
    let user_no_role_id = Uuid::new_v4();
    let user_no_role = NullableUser {
        id: user_no_role_id,
        username: "no_role_user".to_string(),
        role: None,
    };
    db.model::<NullableUser>().insert(&user_no_role).await?;

    let saved_no_role: NullableUser = db.model::<NullableUser>()
        .equals("id", user_no_role_id.to_string())
        .first()
        .await?;
    assert_eq!(saved_no_role.role, None);

    println!("Update to NULL test passed!");
    Ok(())
}
