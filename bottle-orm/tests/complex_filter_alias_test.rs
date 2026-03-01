use bottle_orm::{Database, Model};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Model, PartialEq)]
struct Product {
    #[orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub price: f64,
}

#[tokio::test]
async fn test_complex_filters_with_aliases() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::builder().is_test(true).try_init();
    
    // 1. Setup Database
    let db = Database::builder()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    // 2. Run Migrations
    db.migrator()
        .register::<Product>()
        .run()
        .await?;

    // 3. Insert Test Data
    db.model::<Product>().insert(&Product {
        id: 1,
        name: "Laptop".to_string(),
        price: 1500.0,
    }).await?;

    db.model::<Product>().insert(&Product {
        id: 2,
        name: "Phone".to_string(),
        price: 800.0,
    }).await?;

    db.model::<Product>().insert(&Product {
        id: 3,
        name: "Tablet".to_string(),
        price: 400.0,
    }).await?;

    // 4. Test between with alias
    let results: Vec<Product> = db.model::<Product>()
        .alias("p")
        .between("p.price", 500.0, 1000.0)
        .scan()
        .await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Phone");

    // 5. Test in_list with alias
    let ids_to_find = vec![1, 3];
    let results_in: Vec<Product> = db.model::<Product>()
        .alias("p")
        .in_list("p.id", ids_to_find)
        .order("p.price DESC")
        .scan()
        .await?;

    assert_eq!(results_in.len(), 2);
    assert_eq!(results_in[0].name, "Laptop");
    assert_eq!(results_in[1].name, "Tablet");

    println!("Complex filters with aliases test passed!");
    Ok(())
}
