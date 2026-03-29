# blog-server

Серверная часть блог-платформы. Предоставляет два интерфейса для работы с блогом: HTTP REST API (axum) и gRPC (tonic).

## Возможности

- Регистрация и аутентификация пользователей (JWT + argon2)
- CRUD-операции для постов с проверкой авторства
- Пагинация при просмотре списка постов
- Автоматическое применение SQL-миграций при старте

## Архитектура

Проект следует принципам Clean Architecture:

```
src/
├── domain/          — сущности (User, Post) и ошибки (AppError)
├── data/            — трейты репозиториев и их PostgreSQL-реализации
├── application/     — бизнес-логика (AuthService, BlogService)
├── infrastructure/  — JWT, хеширование паролей, конфигурация, подключение к БД
└── presentation/
    ├── http/        — axum-хендлеры, роутинг, экстрактор аутентификации
    └── grpc/        — tonic-сервис, маппинг из protobuf-типов
```

## Требования

- PostgreSQL 14+
- `protoc` (Protocol Buffers compiler) — `brew install protobuf`

## Переменные окружения

| Переменная     | Описание                    | Значение по умолчанию                     |
|---------------|-----------------------------|-------------------------------------------|
| `DATABASE_URL` | Строка подключения к PostgreSQL | *(обязательная)*                         |
| `JWT_SECRET`   | Секрет для подписи JWT-токенов | `dev-secret-change-in-production`        |
| `HTTP_PORT`    | Порт HTTP-сервера           | `8080`                                    |
| `GRPC_PORT`    | Порт gRPC-сервера           | `50051`                                   |

## Запуск

```bash
# 1. Создать базу данных
createdb blog

# 2. Настроить окружение (или скопировать .env.example в .env)
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/blog
export JWT_SECRET=my-secret-key

# 3. Запустить сервер (миграции применятся автоматически)
cargo run -p blog-server
```

После запуска:
- HTTP API доступен на `http://localhost:8080`
- gRPC доступен на `localhost:50051`

## HTTP API

Все эндпоинты находятся под префиксом `/api`.

### Аутентификация

```bash
# Регистрация
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "email": "alice@example.com", "password": "secret"}'

# Вход
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "secret"}'
```

Оба эндпоинта возвращают JSON с `token` и `user`.

### Посты

Для операций записи (create, update, delete) необходим заголовок `Authorization: Bearer <token>`.

```bash
# Создание поста
curl -X POST http://localhost:8080/api/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"title": "Мой пост", "content": "Содержимое поста"}'

# Получение поста
curl http://localhost:8080/api/posts/1

# Обновление поста (только автор)
curl -X PUT http://localhost:8080/api/posts/1 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"title": "Обновлённый", "content": "Новое содержимое"}'

# Удаление поста (только автор)
curl -X DELETE http://localhost:8080/api/posts/1 \
  -H "Authorization: Bearer <token>"

# Список постов (с пагинацией)
curl "http://localhost:8080/api/posts?limit=10&offset=0"
```

## gRPC API

Определение сервиса находится в `proto/blog.proto`. Методы повторяют HTTP API: `Register`, `Login`, `CreatePost`, `GetPost`, `UpdatePost`, `DeletePost`, `ListPosts`.

Авторизация передаётся через gRPC metadata: ключ `authorization`, значение `Bearer <token>`.

## Тесты

```bash
cargo test -p blog-server
```

Включены unit-тесты для JWT (round-trip, невалидный токен) и хеширования паролей (argon2 hash/verify).
