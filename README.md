# Image Processor With FFI Plugins

CLI-приложение на Rust для обработки PNG-изображений через динамически подключаемые плагины. Проект реализует FFI-взаимодействие, безопасную работу с `unsafe`-кодом и расширяемую плагинную архитектуру в рамках одного Cargo workspace.

## Workspace

```text
practicum-rust/
├── crates/
│   ├── image_processor/  # CLI: загрузка PNG, чтение params, вызов плагина, сохранение PNG
│   ├── mirror_plugin/    # cdylib: зеркальное отражение
│   ├── blur_plugin/      # cdylib: box blur
│   └── e2e-tests/        # сквозные тесты для cargo ci
├── docs/                 # Nextra-документация
└── xtask/                # alias-команды: cargo ci, cargo xtest, cargo docs
```

### Крейты

| Крейт | Тип | Назначение |
| --- | --- | --- |
| `image_processor` | binary + lib | Консольный интерфейс и основной конвейер обработки изображения |
| `mirror_plugin` | `cdylib` | Отражение изображения по горизонтали и/или вертикали |
| `blur_plugin` | `cdylib` | Детерминированное box blur с параметрами радиуса и итераций |
| `e2e-tests` | lib + integration tests | Сквозные проверки CLI и FFI-плагинов |

## Требования

- Rust nightly из `rust-toolchain.toml`
- `cargo-nextest` для тестов устанавливается автоматически через `xtask`
- Bun нужен только для сборки сайта документации

## Команды

```bash
cargo xfmt       # форматирование workspace
cargo xclippy    # clippy с -D warnings
cargo xtest      # unit + e2e + doctests
cargo ci         # fmt-check + clippy + tests
cargo docs       # сборка документации
```

## Как это работает

1. `image_processor` читает PNG через крейт `image`.
2. Изображение преобразуется в `Rgba8` и передаётся как плоский `Vec<u8>`.
3. CLI валидирует `params` как JSON и передаёт их в плагин как `CString`.
4. Плагин загружается через `libloading` из `target/debug` или из директории, переданной в `--plugin-path`.
5. Экспортируемая функция `process_image` модифицирует буфер RGBA на месте.
6. CLI сохраняет результат обратно в PNG.

### ABI плагина

Все плагины экспортируют один и тот же C-совместимый символ:

```c
void process_image(
    uint32_t width,
    uint32_t height,
    uint8_t* rgba_data,
    const char* params
);
```

### Имена библиотек по ОС

- Linux: `libmirror_plugin.so`, `libblur_plugin.so`
- macOS: `libmirror_plugin.dylib`, `libblur_plugin.dylib`
- Windows: `mirror_plugin.dll`, `blur_plugin.dll`

## Форматы params

Оба плагина принимают `params` строго как JSON.

### `mirror_plugin`

```json
{
  "horizontal": true,
  "vertical": false
}
```

Оба поля опциональны. Если оба отсутствуют или равны `false`, плагин делает no-op.

### `blur_plugin`

```json
{
  "radius": 1,
  "iterations": 2
}
```

Оба поля обязательны и должны быть больше нуля.

## Сборка

Собрать все крейты workspace:

```bash
cargo build --workspace
```

После этого артефакты появятся в `target/debug`, включая CLI и динамические библиотеки плагинов.

## Примеры запуска

### Зеркальное отражение

```bash
cat > mirror.json <<'JSON'
{
  "horizontal": true,
  "vertical": false
}
JSON

cargo run -p image_processor -- \
  --input ./input.png \
  --output ./mirrored.png \
  --plugin mirror_plugin \
  --params ./mirror.json \
  --plugin-path ./target/debug
```

### Размытие

```bash
cat > blur.json <<'JSON'
{
  "radius": 1,
  "iterations": 2
}
JSON

cargo run -p image_processor -- \
  --input ./input.png \
  --output ./blurred.png \
  --plugin blur_plugin \
  --params ./blur.json \
  --plugin-path ./target/debug
```

## Тестирование

Локально рекомендуется запускать проверки в таком порядке:

```bash
cargo build --workspace
cargo xtest
cargo xclippy
```

`e2e-tests` ожидают, что `image_processor`, `mirror_plugin` и `blur_plugin` уже собраны в `target/debug`. В `cargo ci` это обеспечивается автоматически через `xtask`.

## Что покрыто тестами

- unit-тесты `image_processor` на ошибки CLI и валидацию `params`
- unit-тесты `mirror_plugin` на горизонтальное, вертикальное и комбинированное отражение
- unit-тесты `blur_plugin` на детерминированный box blur и безопасный no-op при невалидных параметрах
- e2e-тесты с реальным запуском бинарника и загрузкой `cdylib`
