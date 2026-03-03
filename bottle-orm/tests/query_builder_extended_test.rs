use bottle_orm::{Database, Model, Op, FromAnyRow};

#[derive(Debug, Clone, Model, PartialEq)]
struct Product {
    #[orm(primary_key)]
    id: i32,
    name: String,
    category: String,
    price: f64,
    stock: i32,
}

#[derive(Debug, Clone, FromAnyRow, PartialEq)]
struct CategoryDTO {
    category: String,
}

#[derive(Debug, Clone, FromAnyRow, PartialEq)]
struct CategorySummary {
    category: String,
    total_stock: i64,
}

#[tokio::test]
async fn test_query_builder_extended_features() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder().max_connections(1).connect("sqlite::memory:").await?;

    // Register and migrate
    db.migrator().register::<Product>().run().await?;

    // Seed data
    let products = vec![
        Product { id: 1, name: "Laptop".to_string(), category: "Electronics".to_string(), price: 1200.0, stock: 10 },
        Product { id: 2, name: "Smartphone".to_string(), category: "Electronics".to_string(), price: 800.0, stock: 20 },
        Product { id: 3, name: "Coffee Maker".to_string(), category: "Appliances".to_string(), price: 150.0, stock: 5 },
        Product { id: 4, name: "Toaster".to_string(), category: "Appliances".to_string(), price: 50.0, stock: 15 },
        Product { id: 5, name: "Desk Lamp".to_string(), category: "Home".to_string(), price: 30.0, stock: 50 },
    ];

    for p in &products {
        db.model::<Product>().insert(p).await?;
    }

    // 1. Test DISTINCT with DTO
    let categories: Vec<CategoryDTO> = db.model::<Product>()
        .debug()
        .select("category")
        .distinct()
        .scan_as()
        .await?;
    // We expect 3 unique categories
    assert_eq!(categories.len(), 3);
    assert!(categories.iter().any(|c| c.category == "Electronics"));
    assert!(categories.iter().any(|c| c.category == "Appliances"));
    assert!(categories.iter().any(|c| c.category == "Home"));

    // 2. Test UNION
    // Products price > 1000 UNION Products category = 'Home'
    let q1 = db.model::<Product>().debug().filter("price", Op::Gt, 1000.0);
    let q2 = db.model::<Product>().debug().filter("category", Op::Eq, "Home".to_string());
    
    let union_results: Vec<Product> = q1.union(q2).scan().await?;
    // Laptop (>1000) + Desk Lamp ('Home') = 2
    assert_eq!(union_results.len(), 2);
    assert!(union_results.iter().any(|p| p.name == "Laptop"));
    assert!(union_results.iter().any(|p| p.name == "Desk Lamp"));

    // 3. Test SUBQUERY (filter_subquery)
    // Find products in categories that have at least one product with price < 100
    // Subquery: SELECT category FROM product WHERE price < 100
    let subquery = db.model::<Product>().debug().select("category").filter("price", Op::Lt, 100.0);
    
    let subquery_results: Vec<Product> = db.model::<Product>()
        .debug()
        .filter_subquery("category", Op::In, subquery)
        .scan()
        .await?;
    
    // Appliances (Toaster < 100) and Home (Desk Lamp < 100)
    // Results should be: Coffee Maker, Toaster, Desk Lamp
    assert_eq!(subquery_results.len(), 3);
    assert!(subquery_results.iter().any(|p| p.name == "Coffee Maker"));
    assert!(subquery_results.iter().any(|p| p.name == "Toaster"));
    assert!(subquery_results.iter().any(|p| p.name == "Desk Lamp"));

    // 4. Test GROUP BY and HAVING
    // Categories with total stock > 15
    let summary: Vec<CategorySummary> = db.model::<Product>()
        .debug()
        .select("category, SUM(stock) as total_stock")
        .group_by("category")
        .having("SUM(stock)", Op::Gt, 15)
        .scan_as()
        .await?;
    
    // Electronics (10+20=30), Home (50). Appliances (5+15=20)
    // All 3 should be returned if we use > 15
    assert_eq!(summary.len(), 3);
    
    // If we use > 25
    let summary_high: Vec<CategorySummary> = db.model::<Product>()
        .select("category, SUM(stock) as total_stock")
        .group_by("category")
        .having("SUM(stock)", Op::Gt, 25)
        .scan_as()
        .await?;
    assert_eq!(summary_high.len(), 2); // Electronics and Home

    // 5. Test Scalar Aggregations
    let total_count = db.model::<Product>().count().await?;
    assert_eq!(total_count, 5);

    let max_price: f64 = db.model::<Product>().max("price").await?;
    assert_eq!(max_price, 1200.0);

    let min_price: f64 = db.model::<Product>().min("price").await?;
    assert_eq!(min_price, 30.0);

    let avg_price: f64 = db.model::<Product>().avg("price").await?;
    assert_eq!(avg_price, (1200.0 + 800.0 + 150.0 + 50.0 + 30.0) / 5.0);

    let total_stock: i32 = db.model::<Product>().sum("stock").await?;
    assert_eq!(total_stock, 10 + 20 + 5 + 15 + 50);

    // 6. Test TRUNCATE
    db.model::<Product>().truncate().await?;
    let count_after_truncate = db.model::<Product>().count().await?;
    assert_eq!(count_after_truncate, 0);

    println!("Extended QueryBuilder features test passed!");
    Ok(())
}
