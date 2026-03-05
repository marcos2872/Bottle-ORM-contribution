use bottle_orm::{Database, Model, Op};
use serde::{Deserialize, Serialize};

#[derive(Model, Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    #[orm(primary_key)]
    id: i32,
    username: String,

    #[orm(has_many = "Post", foreign_key = "user_id")]
    posts: Vec<Post>,
}

#[derive(Model, Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Post {
    #[orm(primary_key)]
    id: i32,
    user_id: i32,
    title: String,
    status: String,
}

#[tokio::test]
async fn test_with_query_basic() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    db.create_table::<User>().await?;
    db.create_table::<Post>().await?;

    // Seed data
    let user = User { id: 1, username: "user1".to_string(), posts: vec![] };
    db.model::<User>().insert(&user).await?;

    let posts = vec![
        Post { id: 1, user_id: 1, title: "post1".to_string(), status: "draft".to_string() },
        Post { id: 2, user_id: 1, title: "post2".to_string(), status: "published".to_string() },
        Post { id: 3, user_id: 1, title: "post3".to_string(), status: "published".to_string() },
    ];
    db.model::<Post>().batch_insert(&posts).await?;

    // Test with_query: load only published posts, limit 1, order by id desc
    let users = db.model::<User>()
        .with_query("posts", |query: bottle_orm::query_builder::QueryBuilder<bottle_orm::any_struct::AnyImplStruct, bottle_orm::Database>| {
            query
                .filter("status", Op::Eq, "published".to_string())
                .order("id DESC")
                .limit(1)
        })
        .scan_with()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].posts.len(), 1);
    assert_eq!(users[0].posts[0].id, 3); // Should be post 3 (published, highest id)
    assert_eq!(users[0].posts[0].status, "published");

    Ok(())
}
