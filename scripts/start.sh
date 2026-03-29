#!/usr/bin/env bash
set -euo pipefail

# ──────────────────────────────────────────────
# Blog Platform — запуск одной командой
# ──────────────────────────────────────────────
# Поднимает PostgreSQL через Apple Container и запускает сервер.
# Использование:
#   ./scripts/start.sh          — PostgreSQL + blog-server
#   ./scripts/start.sh --down   — остановить и удалить контейнер
#   ./scripts/start.sh --status — показать состояние контейнера

CONTAINER_NAME="blog-postgres"
VOLUME_NAME="blog-pgdata"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

# Проверяет, существует ли и запущен ли контейнер.
# Apple Container inspect возвращает "[]" (пустой массив) для несуществующих контейнеров,
# поэтому проверяем содержимое, а не код возврата.
is_container_running() {
    local output
    output=$(container inspect "$CONTAINER_NAME" 2>/dev/null)
    [ -n "$output" ] && [ "$output" != "[]" ]
}

# ── Остановка ──
if [ "${1:-}" = "--down" ]; then
    echo "Останавливаем PostgreSQL..."
    container stop "$CONTAINER_NAME" 2>/dev/null || true
    container rm "$CONTAINER_NAME" 2>/dev/null || true
    echo "Контейнер $CONTAINER_NAME удалён."
    echo "(Volume $VOLUME_NAME сохранён. Для удаления: container volume rm $VOLUME_NAME)"
    exit 0
fi

# ── Статус ──
if [ "${1:-}" = "--status" ]; then
    if is_container_running; then
        container inspect "$CONTAINER_NAME"
    else
        echo "Контейнер $CONTAINER_NAME не найден."
    fi
    exit 0
fi

# 1. Создать .env если не существует
if [ ! -f .env ]; then
    cp .env.example .env
    echo "Создан .env из .env.example"
fi

# 2. Создать volume (если ещё нет)
if ! container volume inspect "$VOLUME_NAME" 2>/dev/null | grep -q "$VOLUME_NAME"; then
    echo "Создаём volume $VOLUME_NAME..."
    container volume create "$VOLUME_NAME"
fi

# 3. Запустить PostgreSQL (если ещё не запущен)
if is_container_running; then
    echo "PostgreSQL уже запущен ($CONTAINER_NAME)"
else
    echo "Запускаем PostgreSQL через Apple Container..."
    container run \
        --name "$CONTAINER_NAME" \
        -d \
        -e POSTGRES_USER=postgres \
        -e POSTGRES_PASSWORD=postgres \
        -e POSTGRES_DB=blog \
        -p 5432:5432 \
        -v "$VOLUME_NAME:/var/lib/postgresql/data" \
        docker.io/library/postgres:17-alpine

    # Ждём готовности PostgreSQL
    echo -n "Ожидаем готовность PostgreSQL"
    for i in $(seq 1 30); do
        if container exec "$CONTAINER_NAME" pg_isready -U postgres &>/dev/null; then
            echo " готов!"
            break
        fi
        echo -n "."
        sleep 1
        if [ "$i" -eq 30 ]; then
            echo " таймаут!"
            echo "Проверьте логи: container logs $CONTAINER_NAME"
            exit 1
        fi
    done
fi

echo "PostgreSQL доступен на localhost:5432"

# 4. Собрать и запустить сервер
echo ""
echo "Собираем и запускаем blog-server..."
echo "  HTTP API:  http://localhost:8080"
echo "  gRPC:      localhost:50051"
echo ""
cargo run -p blog-server
