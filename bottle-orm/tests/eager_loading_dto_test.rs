use bottle_orm::{Database, Model, Op};
use serde::{Deserialize, Serialize};

// Original Models
#[derive(Model, Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    #[orm(primary_key)]
    id: i32,
    username: String,
    password: String, // Sensível, não deve ir pro DTO
}

#[derive(Model, Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Post {
    #[orm(primary_key)]
    id: i32,
    user_id: i32,
    title: String,
    content: String, // Pode ser muito grande
    status: String,
}

// DTOs
#[derive(Model, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[orm(table = "user")] // Aponta para a tabela 'user'
struct UserDTO {
    #[orm(primary_key)]
    id: i32,
    username: String,
    
    #[orm(has_many = "PostDTO", foreign_key = "user_id")]
    posts: Vec<PostDTO>,
}

#[derive(Model, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[orm(table = "post")] // Aponta para a tabela 'post'
struct PostDTO {
    #[orm(primary_key)]
    id: i32,
    user_id: i32,
    title: String,
    // Note: content e status omitidos
}

#[tokio::test]
async fn test_eager_loading_with_dtos() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::builder()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    db.create_table::<User>().await?;
    db.create_table::<Post>().await?;

    // Seed data
    let user = User { id: 1, username: "john".to_string(), password: "secret_hash".to_string() };
    db.model::<User>().insert(&user).await?;

    let posts = vec![
        Post { id: 1, user_id: 1, title: "Post 1".to_string(), content: "...".to_string(), status: "published".to_string() },
        Post { id: 2, user_id: 1, title: "Post 2".to_string(), content: "...".to_string(), status: "draft".to_string() },
    ];
    db.model::<Post>().batch_insert(&posts).await?;

    // Use scan_as_with to load UserDTO and eager load PostDTO with filters
    let users_dto = db.model::<User>()
        .with_query("posts", |query| {
            query
                .filter("status", Op::Eq, "published".to_string())
        })
        .scan_as_with::<UserDTO>()
        .await?;

    assert_eq!(users_dto.len(), 1);
    let u = &users_dto[0];
    assert_eq!(u.username, "john");
    // u.password não existe no DTO, então não foi carregado
    
    assert_eq!(u.posts.len(), 1); // Apenas o 'published'
    assert_eq!(u.posts[0].title, "Post 1");
    // u.posts[0].content não existe no DTO

    Ok(())
}
