# quote-client

Клиент для получения потока биржевых котировок от `quote-server`. Подписывается на выбранные тикеры и выводит полученные котировки в лог.

## Запуск

### 1. Подготовьте файл с тикерами

Создайте текстовый файл с интересующими тикерами (по одному на строку):

```bash
cat > my_tickers.txt << 'EOF'
AAPL
TSLA
GOOGL
NVDA
EOF
```

Доступные тикеры определяются сервером (110 акций: AAPL, MSFT, GOOGL, AMZN, NVDA, META, TSLA, JPM и др.)

### 2. Убедитесь, что сервер запущен

```bash
RUST_LOG=info cargo run -p quote-server
```

### 3. Запустите клиент

```bash
RUST_LOG=info cargo run -p quote-client -- \
    --server-addr 127.0.0.1:8080 \
    --udp-port 34254 \
    --tickers-file my_tickers.txt
```

## Аргументы командной строки

| Аргумент         | Обязательный | Описание                                      |
|------------------|:------------:|-----------------------------------------------|
| `--server-addr`  | да           | TCP-адрес сервера котировок (`host:port`)      |
| `--udp-port`     | да           | Локальный UDP-порт для приёма котировок        |
| `--tickers-file` | да           | Путь к файлу с тикерами (по одному на строку) |

## Переменные окружения

| Переменная | Описание                          | Пример                        |
|------------|-----------------------------------|-------------------------------|
| `RUST_LOG`  | Уровень логирования (`tracing`)  | `info`, `debug`, `quote_client=debug` |

## Формат файла тикеров

Обычный текстовый файл, по одному тикеру на строку. Пустые строки и пробелы по краям игнорируются. Регистр не важен — тикеры приводятся к верхнему автоматически.

```
AAPL
tsla
  googl
```

## Пример вывода

```
2025-01-15T10:00:00.123Z  INFO quote_client: quote received ticker=AAPL price=187.42 volume=3421
2025-01-15T10:00:00.124Z  INFO quote_client: quote received ticker=TSLA price=242.50 volume=8754
2025-01-15T10:00:00.223Z  INFO quote_client: quote received ticker=GOOGL price=141.88 volume=1520
```

## Завершение работы

Нажмите `Ctrl+C` — клиент корректно завершит все потоки и закроет соединение.

## Быстрый старт (сервер + клиент)

Терминал 1 — запуск сервера:

```bash
RUST_LOG=info cargo run -p quote-server
```

Терминал 2 — запуск клиента:

```bash
echo -e "AAPL\nTSLA\nNVDA" > /tmp/tickers.txt

RUST_LOG=info cargo run -p quote-client -- \
    --server-addr 127.0.0.1:8080 \
    --udp-port 9000 \
    --tickers-file /tmp/tickers.txt
```