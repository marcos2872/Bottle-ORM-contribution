use bottle_orm::{Database, Model};
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

    #[orm(has_many = "Comment", foreign_key = "post_id")]
    comments: Vec<Comment>,

    #[orm(belongs_to = "User", foreign_key = "user_id")]
    user: Option<User>,
}

#[derive(Model, Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Comment {
    #[orm(primary_key)]
    id: i32,
    post_id: i32,
    content: String,
}

#[tokio::test]
async fn test_nested_eager_loading() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    db.create_table::<User>().await?;
    db.create_table::<Post>().await?;
    db.create_table::<Comment>().await?;

    // Seed data
    let user = User { id: 1, username: "user1".to_string(), posts: vec![] };
    db.model::<User>().insert(&user).await?;

    let post1 = Post { id: 1, user_id: 1, title: "post1".to_string(), comments: vec![], user: None };
    let post2 = Post { id: 2, user_id: 1, title: "post2".to_string(), comments: vec![], user: None };
    db.model::<Post>().insert(&post1).await?;
    db.model::<Post>().insert(&post2).await?;

    let comment1 = Comment { id: 1, post_id: 1, content: "comment1".to_string() };
    let comment2 = Comment { id: 2, post_id: 1, content: "comment2".to_string() };
    let comment3 = Comment { id: 3, post_id: 2, content: "comment3".to_string() };
    db.model::<Comment>().insert(&comment1).await?;
    db.model::<Comment>().insert(&comment2).await?;
    db.model::<Comment>().insert(&comment3).await?;

    // Test nested eager loading: User -> Posts -> Comments
    let users = db.model::<User>()
        .with("posts.comments")
        .scan_with()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].posts.len(), 2);
    
    // Check comments for post 1
    let p1 = users[0].posts.iter().find(|p| p.id == 1).unwrap();
    assert_eq!(p1.comments.len(), 2);
    assert!(p1.comments.iter().any(|c| c.content == "comment1"));
    assert!(p1.comments.iter().any(|c| c.content == "comment2"));

    // Check comments for post 2
    let p2 = users[0].posts.iter().find(|p| p.id == 2).unwrap();
    assert_eq!(p2.comments.len(), 1);
    assert_eq!(p2.comments[0].content, "comment3");

    // Test multiple with clauses including nested ones
    let users_multi = db.model::<User>()
        .with("posts.comments")
        .with("posts.user") // Each post also gets its user (circular but works)
        .scan_with()
        .await?;

    assert_eq!(users_multi.len(), 1);
    assert_eq!(users_multi[0].posts.len(), 2);
    assert!(users_multi[0].posts[0].comments.len() > 0);
    assert!(users_multi[0].posts[0].user.is_some());
    assert_eq!(users_multi[0].posts[0].user.as_ref().unwrap().id, 1);

    // Test nested eager loading with belongs_to: Post -> User -> Posts
    // This is a bit circular but valid for testing
    let posts = db.model::<Post>()
        .with("user.posts")
        .scan_with()
        .await?;

    assert_eq!(posts.len(), 2);
    for post in posts {
        assert!(post.user.is_some());
        let u = post.user.unwrap();
        assert_eq!(u.posts.len(), 2); // The user has 2 posts
    }

    // Test relation options: limit and order
    let users_opt = db.model::<User>()
        .with("posts[limit=1,order=id DESC]")
        .scan_with()
        .await?;

    assert_eq!(users_opt.len(), 1);
    // Even though the user has 2 posts, we limited the TOTAL fetch to 1
    // and ordered by ID DESC, so it should be post 2.
    assert_eq!(users_opt[0].posts.len(), 1);
    assert_eq!(users_opt[0].posts[0].id, 2);

    Ok(())
}
