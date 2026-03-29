# blog-cli

Консольный клиент для блог-платформы. Обёртка над `blog-client` с интерфейсом на базе clap.

## Возможности

- Регистрация и вход с автоматическим сохранением токена
- Полный CRUD для постов из терминала
- Поддержка HTTP и gRPC транспортов
- Настраиваемый адрес сервера

## Установка

```bash
cargo build -p blog-cli --release
# Бинарник: target/release/blog-cli
```

## Использование

### Глобальные опции

| Опция          | Описание                                      | По умолчанию             |
|----------------|-----------------------------------------------|--------------------------|
| `--grpc`       | Использовать gRPC вместо HTTP                  | HTTP                     |
| `--server, -s` | Адрес сервера                                  | `http://localhost:8080` (HTTP) / `http://localhost:50051` (gRPC) |

### Аутентификация

Токен автоматически сохраняется в файл `.blog_token` в текущей директории после регистрации или входа. Все последующие команды используют этот токен.

```bash
# Регистрация
cargo run -p blog-cli -- register \
  --username alice \
  --email alice@example.com \
  --password secret

# Вход
cargo run -p blog-cli -- login \
  --username alice \
  --password secret
```

### Работа с постами

```bash
# Создание поста
cargo run -p blog-cli -- create \
  --title "Мой первый пост" \
  --content "Привет, мир!"

# Просмотр поста по ID
cargo run -p blog-cli -- get --id 1

# Обновление поста
cargo run -p blog-cli -- update \
  --id 1 \
  --title "Обновлённый заголовок" \
  --content "Новое содержимое"

# Удаление поста
cargo run -p blog-cli -- delete --id 1

# Список постов (с пагинацией)
cargo run -p blog-cli -- list --limit 10 --offset 0
```

### Работа через gRPC

```bash
# Все команды работают аналогично с флагом --grpc
cargo run -p blog-cli -- --grpc register \
  --username alice \
  --email alice@example.com \
  --password secret

# С кастомным адресом
cargo run -p blog-cli -- --grpc --server http://my-server:50051 list
```

## Хранение токена

Файл `.blog_token` создаётся в текущей рабочей директории. Он содержит JWT-токен в текстовом виде. Файл включён в `.gitignore`.

Для переключения между пользователями достаточно выполнить `login` с другими учётными данными.
