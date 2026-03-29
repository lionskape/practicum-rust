# blog-client

Библиотека-клиент для блог-платформы. Поддерживает два транспорта: HTTP (reqwest) и gRPC (tonic), предоставляя единый интерфейс через `BlogClient`.

## Возможности

- Регистрация и аутентификация
- CRUD-операции для постов
- Автоматическое управление JWT-токеном (сохраняется внутри клиента)
- Переключение между HTTP и gRPC без изменения вызывающего кода

## Установка

Крейт является частью workspace и подключается по пути:

```toml
[dependencies]
blog-client = { path = "../blog-client" }
```

## Использование

```rust
use blog_client::{BlogClient, Transport};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // HTTP-транспорт
    let mut client = BlogClient::new(
        Transport::Http("http://localhost:8080".to_string())
    ).await?;

    // Или gRPC-транспорт
    // let mut client = BlogClient::new(
    //     Transport::Grpc("http://localhost:50051".to_string())
    // ).await?;

    // Регистрация (токен сохраняется автоматически при использовании CLI,
    // здесь нужно вызвать set_token вручную)
    let auth = client.register("alice", "alice@example.com", "password").await?;
    client.set_token(auth.token);

    // Создание поста
    let post = client.create_post("Заголовок", "Содержимое").await?;
    println!("Создан пост: {} (id: {})", post.title, post.id);

    // Список постов
    let list = client.list_posts(Some(10), Some(0)).await?;
    println!("Всего постов: {}", list.total);

    // Получение конкретного поста
    let post = client.get_post(1).await?;

    // Обновление поста
    let updated = client.update_post(1, "Новый заголовок", "Новое содержимое").await?;

    // Удаление поста
    client.delete_post(1).await?;

    Ok(())
}
```

## Структура модулей

| Модуль         | Описание                                              |
|----------------|-------------------------------------------------------|
| `lib.rs`       | `BlogClient` — единый фасад с enum dispatch по транспорту |
| `http_client`  | `HttpBlogClient` — реализация через reqwest           |
| `grpc_client`  | `GrpcBlogClient` — реализация через tonic             |
| `types`        | Общие типы: `AuthResponse`, `Post`, `ListPostsResponse` |
| `error`        | `BlogClientError` — ошибки клиента (HTTP, gRPC, сериализация) |

## Обработка ошибок

Все методы возвращают `Result<T, BlogClientError>`. Варианты ошибок:

- `Http` — ошибка reqwest (сеть, таймаут)
- `Grpc` — ошибка tonic (статус-код gRPC)
- `Api` — ошибка от сервера (4xx/5xx с телом ответа)
- `Deserialization` — ошибка десериализации JSON
