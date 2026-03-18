use bottle_orm::{Database, Model, Op};

// ============================================================================
// Shared model
// ============================================================================

#[derive(Debug, Clone, Model, PartialEq)]
struct Item {
    #[orm(primary_key)]
    id: i32,
    name: String,
    #[orm(nullable)]
    description: Option<String>,
    stock: i32,
}

async fn setup_db() -> Result<Database, Box<dyn std::error::Error>> {
    let db = Database::builder().max_connections(1).connect("sqlite::memory:").await?;
    db.migrator().register::<Item>().run().await?;
    Ok(db)
}

async fn seed(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    db.model::<Item>().insert(&Item { id: 1, name: "Hammer".into(), description: Some("A tool".into()), stock: 10 }).await?;
    db.model::<Item>().insert(&Item { id: 2, name: "Nail".into(), description: None, stock: 100 }).await?;
    db.model::<Item>().insert(&Item { id: 3, name: "Screwdriver".into(), description: Some("Phillips".into()), stock: 5 }).await?;
    Ok(())
}

// ============================================================================
// is_null / is_not_null
// ============================================================================

#[tokio::test]
async fn test_is_null_filters_null_rows() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;
    seed(&db).await?;

    let items: Vec<Item> = db.model::<Item>().is_null("description").scan().await?;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Nail");
    Ok(())
}

#[tokio::test]
async fn test_is_not_null_filters_non_null_rows() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;
    seed(&db).await?;

    let items: Vec<Item> = db.model::<Item>().is_not_null("description").scan().await?;
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|i| i.description.is_some()));
    Ok(())
}

#[tokio::test]
async fn test_is_null_combined_with_filter() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;
    seed(&db).await?;

    // Null description AND stock >= 50
    let items: Vec<Item> = db.model::<Item>()
        .is_null("description")
        .filter("stock", Op::Gte, 50)
        .scan()
        .await?;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Nail");
    Ok(())
}

#[tokio::test]
async fn test_is_null_no_matches() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;
    seed(&db).await?;

    // No rows have null name
    let items: Vec<Item> = db.model::<Item>().is_null("name").scan().await?;
    assert_eq!(items.len(), 0);
    Ok(())
}

// ============================================================================
// hard_delete
// ============================================================================

#[tokio::test]
async fn test_hard_delete_with_filter() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;
    seed(&db).await?;

    let affected = db.model::<Item>()
        .filter("id", Op::Eq, 1)
        .hard_delete()
        .await?;
    assert_eq!(affected, 1);

    let remaining: Vec<Item> = db.model::<Item>().scan().await?;
    assert_eq!(remaining.len(), 2);
    assert!(remaining.iter().all(|i| i.id != 1));
    Ok(())
}

#[tokio::test]
async fn test_hard_delete_no_match() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;
    seed(&db).await?;

    let affected = db.model::<Item>()
        .filter("id", Op::Eq, 999)
        .hard_delete()
        .await?;
    assert_eq!(affected, 0);

    let remaining: Vec<Item> = db.model::<Item>().scan().await?;
    assert_eq!(remaining.len(), 3);
    Ok(())
}

#[tokio::test]
async fn test_hard_delete_all_rows() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;
    seed(&db).await?;

    // No filter = deletes everything
    let affected = db.model::<Item>().hard_delete().await?;
    assert_eq!(affected, 3);

    let remaining: Vec<Item> = db.model::<Item>().scan().await?;
    assert_eq!(remaining.len(), 0);
    Ok(())
}

// ============================================================================
// omit
// ============================================================================

#[derive(Debug, Clone, PartialEq, bottle_orm::FromAnyRow)]
struct ItemWithoutStock {
    id: i32,
    name: String,
}

#[tokio::test]
async fn test_omit_excludes_column_from_select() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;
    seed(&db).await?;

    let items: Vec<ItemWithoutStock> = db.model::<Item>()
        .omit("stock, description")
        .scan_as()
        .await?;
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].id, 1);
    assert_eq!(items[0].name, "Hammer");
    Ok(())
}

// ============================================================================
// Transaction — commit
// ============================================================================

#[tokio::test]
async fn test_transaction_commit_persists_data() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;

    let tx = db.begin().await?;
    tx.model::<Item>().insert(&Item { id: 10, name: "Bolt".into(), description: None, stock: 50 }).await?;
    tx.commit().await?;

    let items: Vec<Item> = db.model::<Item>().filter("id", Op::Eq, 10).scan().await?;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Bolt");
    Ok(())
}

// ============================================================================
// Transaction — rollback
// ============================================================================

#[tokio::test]
async fn test_transaction_rollback_discards_data() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;

    let tx = db.begin().await?;
    tx.model::<Item>().insert(&Item { id: 20, name: "Ghost".into(), description: None, stock: 1 }).await?;
    tx.rollback().await?;

    let items: Vec<Item> = db.model::<Item>().filter("id", Op::Eq, 20).scan().await?;
    assert_eq!(items.len(), 0, "rollback must discard the insert");
    Ok(())
}

#[tokio::test]
async fn test_transaction_rollback_does_not_affect_existing_data() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;
    seed(&db).await?;

    let tx = db.begin().await?;
    tx.model::<Item>().filter("id", Op::Eq, 1).hard_delete().await?;
    tx.rollback().await?;

    // The delete was rolled back, so all 3 rows should still exist
    let items: Vec<Item> = db.model::<Item>().scan().await?;
    assert_eq!(items.len(), 3);
    Ok(())
}

// ============================================================================
// updates (full model update)
// ============================================================================

#[tokio::test]
async fn test_updates_full_model() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_db().await?;
    seed(&db).await?;

    let updated = Item { id: 1, name: "Big Hammer".into(), description: Some("Updated".into()), stock: 99 };
    db.model::<Item>()
        .filter("id", Op::Eq, 1)
        .updates(&updated)
        .await?;

    let item: Item = db.model::<Item>().filter("id", Op::Eq, 1).first().await?;
    assert_eq!(item.name, "Big Hammer");
    assert_eq!(item.stock, 99);
    assert_eq!(item.description, Some("Updated".into()));
    Ok(())
}
