# Blog Platform

Полнофункциональная блог-платформа на Rust, состоящая из 4 крейтов: сервер с двумя API (HTTP + gRPC), клиентская библиотека, CLI и WASM-фронтенд.

## Архитектура

```
┌─────────────────────────────────────────────────────────┐
│                    blog-server                          │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │  axum    │  │    tonic     │  │   PostgreSQL     │  │
│  │ HTTP API │  │  gRPC API    │  │   (sqlx)         │  │
│  │ :8080    │  │  :50051      │  │                  │  │
│  └────┬─────┘  └──────┬───────┘  └────────┬─────────┘  │
│       │               │                   │             │
│       └───────┬───────┘                   │             │
│               ▼                           │             │
│  ┌──────────────────────┐  ┌──────────────┘             │
│  │   Application Layer  │  │                            │
│  │  AuthService         │  │  Domain: User, Post        │
│  │  BlogService         │  │  Data:   Repositories      │
│  └──────────────────────┘  └────────────────────────────│
└─────────────────────────────────────────────────────────┘
        ▲                           ▲
        │ HTTP (reqwest)            │ gRPC (tonic)
        │                           │
┌───────┴───────────────────────────┴──────┐
│              blog-client                  │
│  Unified API: Transport::Http | ::Grpc   │
└──────────────────┬───────────────────────┘
        ▲          │
        │          ▼
┌───────┴──┐  ┌────────────┐
│ blog-cli │  │ blog-wasm  │
│  (clap)  │  │ (gloo-net) │
│ Terminal │  │  Browser   │
└──────────┘  └────────────┘
```

### Крейты

| Крейт | Тип | Описание |
|-------|-----|----------|
| [`blog-server`](crates/blog-server/) | binary | HTTP + gRPC сервер, Clean Architecture, JWT auth, PostgreSQL |
| [`blog-client`](crates/blog-client/) | library | Клиентская библиотека с HTTP и gRPC транспортами |
| [`blog-cli`](crates/blog-cli/) | binary | Консольный клиент (clap), работает через `blog-client` |
| [`blog-wasm`](crates/blog-wasm/) | cdylib | Браузерный SPA, прямые HTTP-запросы через `gloo-net` |

### Связи между крейтами

- **blog-cli** зависит от **blog-client** (использует `BlogClient` API)
- **blog-client** общается с **blog-server** по HTTP или gRPC
- **blog-wasm** общается с **blog-server** напрямую по HTTP (не зависит от `blog-client`, т.к. `reqwest`/`tonic` не работают в WASM)
- **blog-server** — самостоятельный, не зависит от остальных крейтов

## Требования

- **Rust nightly** (устанавливается автоматически через `rust-toolchain.toml`)
- **[Apple Container](https://github.com/apple/container)** (для PostgreSQL):
  ```bash
  brew install container
  ```
- **protoc** (Protocol Buffers compiler):
  ```bash
  brew install protobuf
  ```

## Быстрый старт

Одна команда для запуска PostgreSQL + сервера:

```bash
./scripts/start.sh
```

Скрипт:
1. Создаёт `.env` из `.env.example` (если нет)
2. Создаёт volume `blog-pgdata` для персистентного хранения данных
3. Запускает PostgreSQL 17 в Apple Container и ждёт готовности
4. Собирает и запускает `blog-server` (миграции применяются автоматически)

После запуска:
- HTTP API: http://localhost:8080
- gRPC: localhost:50051

Управление:
```bash
./scripts/start.sh          # запуск PostgreSQL + сервер
./scripts/start.sh --down   # остановить и удалить контейнер (данные сохраняются в volume)
./scripts/start.sh --status # показать состояние контейнера
```

### Ручной запуск (без скрипта)

```bash
# 1. Запустить PostgreSQL через Apple Container
container run --name blog-postgres -d \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=blog \
  -p 5432:5432 \
  docker.io/library/postgres:17-alpine

# 2. Создать .env
cp .env.example .env

# 3. Запустить сервер
cargo run -p blog-server

# Остановка
container stop blog-postgres
container rm blog-postgres
```

## Переменные окружения

Файл `.env` (создаётся из `.env.example`):

| Переменная | Описание | По умолчанию |
|-----------|----------|--------------|
| `DATABASE_URL` | Строка подключения к PostgreSQL | `postgres://postgres:postgres@localhost:5432/blog` |
| `JWT_SECRET` | Секрет для подписи JWT-токенов | `change-me-in-production` |
| `HTTP_PORT` | Порт HTTP API | `8080` |
| `GRPC_PORT` | Порт gRPC API | `50051` |

Для продакшена замените `JWT_SECRET` на случайную строку:

```bash
openssl rand -base64 32
```

## Сценарии использования

После запуска сервера (`./scripts/start.sh`) можно работать через curl, CLI или браузер.

### Сценарий 1: curl (HTTP API)

Все эндпоинты находятся под префиксом `/api`.

```bash
# --- Регистрация ---
curl -s -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "email": "alice@example.com", "password": "secret123"}' \
  | jq .

# Ответ:
# {
#   "token": "eyJhbGciOiJIUzI1NiIs...",
#   "user": { "id": 1, "username": "alice", "email": "alice@example.com" }
# }

# Сохраним токен в переменную:
TOKEN=$(curl -s -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username": "bob", "email": "bob@example.com", "password": "secret123"}' \
  | jq -r .token)

# --- Вход ---
TOKEN=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "bob", "password": "secret123"}' \
  | jq -r .token)

# --- Создание поста ---
curl -s -X POST http://localhost:8080/api/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"title": "Мой первый пост", "content": "Привет, мир! Это блог на Rust."}' \
  | jq .

# --- Список постов (пагинация) ---
curl -s "http://localhost:8080/api/posts?limit=10&offset=0" | jq .

# --- Получение поста по ID ---
curl -s http://localhost:8080/api/posts/1 | jq .

# --- Обновление поста (только автор) ---
curl -s -X PUT http://localhost:8080/api/posts/1 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"title": "Обновлённый заголовок", "content": "Новое содержимое поста."}' \
  | jq .

# --- Удаление поста (только автор) ---
curl -s -X DELETE http://localhost:8080/api/posts/1 \
  -H "Authorization: Bearer $TOKEN" -w "\nHTTP %{http_code}\n"
# Ответ: HTTP 204 (No Content)
```

### Сценарий 2: CLI (blog-cli)

```bash
# --- Регистрация (токен сохраняется в .blog_token) ---
cargo run -p blog-cli -- register \
  --username alice \
  --email alice@example.com \
  --password secret123

# Registered successfully!
# User: alice (id: 1, email: alice@example.com)

# --- Вход ---
cargo run -p blog-cli -- login \
  --username alice \
  --password secret123

# --- Создание поста ---
cargo run -p blog-cli -- create \
  --title "Пост из CLI" \
  --content "Создан через консольный клиент"

# --- Просмотр поста ---
cargo run -p blog-cli -- get --id 1

# --- Список постов ---
cargo run -p blog-cli -- list --limit 10

# --- Обновление поста ---
cargo run -p blog-cli -- update --id 1 \
  --title "Обновлённый" \
  --content "Изменено через CLI"

# --- Удаление поста ---
cargo run -p blog-cli -- delete --id 1

# --- Работа через gRPC (вместо HTTP) ---
cargo run -p blog-cli -- --grpc register \
  --username charlie \
  --email charlie@example.com \
  --password secret123

# --- Указание адреса сервера ---
cargo run -p blog-cli -- --server http://my-server:8080 list
```

### Сценарий 3: WASM (браузер)

```bash
# 1. Установить wasm-pack
cargo install wasm-pack

# 2. Собрать WASM-модуль
wasm-pack build --target web crates/blog-wasm

# 3. Запустить локальный HTTP-сервер
cd crates/blog-wasm
python3 -m http.server 8000
```

Откройте http://localhost:8000 в браузере:

1. **Регистрация** — заполните поля Username, Email, Password, нажмите Register
2. **Вход** — или используйте Login с существующим аккаунтом
3. **Создание поста** — после входа появятся поля Title и Content
4. **Просмотр ленты** — посты отображаются автоматически, кнопка Refresh обновляет
5. **Выход** — кнопка Logout очищает токен из localStorage

## HTTP API Reference

| Метод | Эндпоинт | Auth | Описание | Статус |
|-------|----------|------|----------|--------|
| POST | `/api/auth/register` | — | Регистрация | 201 |
| POST | `/api/auth/login` | — | Вход | 200 |
| POST | `/api/posts` | Bearer | Создать пост | 201 |
| GET | `/api/posts` | — | Список постов (?limit=&offset=) | 200 |
| GET | `/api/posts/{id}` | — | Получить пост | 200 |
| PUT | `/api/posts/{id}` | Bearer | Обновить пост (автор) | 200 |
| DELETE | `/api/posts/{id}` | Bearer | Удалить пост (автор) | 204 |

### Коды ошибок

| Код | Причина |
|-----|---------|
| 401 | Неверные учётные данные / отсутствует токен |
| 403 | Попытка изменить/удалить чужой пост |
| 404 | Пост или пользователь не найден |
| 409 | Пользователь с таким username/email уже существует |

## gRPC API

Определение сервиса: [`proto/blog.proto`](crates/blog-server/proto/blog.proto)

Методы: `Register`, `Login`, `CreatePost`, `GetPost`, `UpdatePost`, `DeletePost`, `ListPosts`.

Авторизация через metadata: ключ `authorization`, значение `Bearer <token>`.

## Структура проекта

```
practicum-rust/
├── Cargo.toml              — workspace с общими зависимостями
├── rust-toolchain.toml     — nightly Rust
├── scripts/
│   └── start.sh            — запуск одной командой (Apple Container + сервер)
├── .env.example            — шаблон переменных окружения
├── crates/
│   ├── blog-server/        — сервер (axum + tonic + sqlx)
│   │   ├── proto/          — gRPC-определения (blog.proto)
│   │   ├── migrations/     — SQL-миграции (users, posts)
│   │   └── src/
│   │       ├── domain/     — сущности и ошибки
│   │       ├── data/       — трейты репозиториев + PostgreSQL
│   │       ├── application/— бизнес-логика (AuthService, BlogService)
│   │       ├── infrastructure/ — JWT, пароли, конфиг, БД
│   │       └── presentation/   — HTTP (axum) + gRPC (tonic)
│   ├── blog-client/        — клиентская библиотека (reqwest + tonic)
│   ├── blog-cli/           — консольный клиент (clap)
│   └── blog-wasm/          — браузерный SPA (wasm-bindgen + gloo-net)
├── xtask/                  — CI-автоматизация (fmt, clippy, test)
└── docs/                   — документация (Nextra)
```

## Сборка и тесты

```bash
# Полный CI pipeline (формат + линтер + тесты)
cargo ci

# Только тесты
cargo test --workspace

# Только clippy
cargo clippy --workspace -- -D warnings

# Форматирование
cargo fmt --all
```

## Технологический стек

| Компонент | Технология |
|-----------|------------|
| HTTP-сервер | axum 0.8 |
| gRPC-сервер | tonic 0.13 + prost |
| База данных | PostgreSQL 17 + sqlx 0.8 |
| Контейнеризация | [Apple Container](https://github.com/apple/container) |
| Аутентификация | JWT (jsonwebtoken) + argon2 |
| HTTP-клиент | reqwest 0.12 |
| gRPC-клиент | tonic 0.13 |
| CLI | clap 4 (derive) |
| WASM | wasm-bindgen + gloo-net + web-sys |
| CI | xtask (cargo ci = fmt + clippy + nextest) |
