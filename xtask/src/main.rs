//! # xtask - Автоматизация сборки проекта
//!
//! Этот крейт предоставляет команды автоматизации сборки для воркспейса.
//!
//! См. [`HELP_TEXT`] для полного списка доступных команд и информации по использованию.
use std::fs;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use xshell::{Shell, cmd};

/// Текст справки для команды xtask.
///
/// Эта константа содержит полное сообщение справки, которое отображается
/// при запуске `cargo run -p xtask -- help`.
pub const HELP_TEXT: &str = r#"xtask

Использование:
  cargo run -p xtask -- <команда>

Команды:
  help         Показать это сообщение
  fmt          Запустить rustfmt
  fmt-check    Проверить форматирование (CI)
  clippy       Запустить clippy (воркспейс)
  test         Запустить тесты через nextest (воркспейс)
  ci           Запустить fmt-check + clippy + test (профиль CI)
  docs         Собрать документацию (rustdoc JSON + Nextra)
  docs-dev     Запустить dev сервер Nextra
  docs-rustdoc Сгенерировать API документацию из rustdoc JSON

Примечание:
  cargo-nextest устанавливается автоматически при первом запуске тестов
"#;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "help".to_string());

    let sh = Shell::new()?;

    match cmd.as_str() {
        "help" | "-h" | "--help" => help(),
        "fmt" => Ok(cmd!(sh, "cargo +nightly fmt --all").run()?),
        "fmt-check" => Ok(cmd!(sh, "cargo +nightly fmt --all -- --check").run()?),
        "clippy" => Ok(cmd!(sh, "cargo +nightly clippy --workspace -- -D warnings").run()?),
        "test" => {
            ensure_nextest(&sh)?;
            cmd!(sh, "cargo nextest run --workspace").run()?;
            // Run doctests separately (nextest doesn't support them)
            cmd!(sh, "cargo +nightly test --workspace --doc").run()?;
            Ok(())
        }
        "ci" => {
            ensure_nextest(&sh)?;
            cmd!(sh, "cargo +nightly fmt --all -- --check").run()?;
            cmd!(sh, "cargo +nightly clippy --workspace -- -D warnings").run()?;
            cmd!(sh, "cargo nextest run --workspace --profile ci").run()?;
            // Run doctests separately (nextest doesn't support them)
            cmd!(sh, "cargo +nightly test --workspace --doc").run()?;
            Ok(())
        }
        "docs" => docs_build(),
        "docs-dev" => docs_dev(),
        "docs-rustdoc" => docs_rustdoc(),
        other => bail!("Неизвестная команда: {other}\n\nЗапустите: cargo run -p xtask -- help"),
    }
}

/// Показать сообщение справки.
///
/// Эта функция выводит текст справки в stdout, показывая все доступные
/// команды и их описания.
fn help() -> Result<()> {
    println!("{}", HELP_TEXT);
    Ok(())
}

/// Собрать полную документацию.
///
/// Эта команда выполняет следующие шаги:
/// 1. Генерирует rustdoc JSON для всех крейтов воркспейса
/// 2. Конвертирует JSON в Markdown через rustdoc-md
/// 3. Устанавливает зависимости Nextra
/// 4. Собирает статический сайт документации
///
/// Итоговая документация будет доступна в `docs/out/`.
fn docs_build() -> Result<()> {
    let sh = Shell::new()?;
    let docs_dir = project_root()?.join("docs");

    // Генерация rustdoc JSON -> Markdown
    docs_rustdoc()?;

    // Установка зависимостей
    sh.change_dir(&docs_dir);
    cmd!(sh, "bun install").run()?;

    // Сборка статического сайта Nextra
    cmd!(sh, "bun run build").run()?;

    eprintln!("Документация успешно собрана в docs/out/");
    Ok(())
}

/// Запустить dev сервер Nextra.
///
/// Эта команда:
/// 1. Устанавливает зависимости Nextra при необходимости
/// 2. Запускает dev сервер с hot-reload
///
/// Документация будет доступна по адресу http://localhost:3000.
fn docs_dev() -> Result<()> {
    let sh = Shell::new()?;
    let docs_dir = project_root()?.join("docs");

    // Установка зависимостей
    sh.change_dir(&docs_dir);
    cmd!(sh, "bun install").run()?;

    // Запуск dev сервера Nextra
    cmd!(sh, "bun run dev").run()?;
    Ok(())
}

/// Сгенерировать API документацию из rustdoc JSON.
///
/// Эта команда:
/// 1. Генерирует rustdoc JSON для всех крейтов воркспейса через nightly Rust
/// 2. Конвертирует JSON в Markdown через API библиотеки rustdoc-md
/// 3. Удаляет строки заголовка перед первым "# " хедингом
///
/// Сгенерированная документация будет размещена в `docs/content/api/`.
///
/// # Требования
///
/// - Rust nightly toolchain
fn docs_rustdoc() -> Result<()> {
    let sh = Shell::new()?;
    let project = project_root()?;
    let api_dir = project.join("docs/content/api");

    // Создание директории api_dir
    fs::create_dir_all(&api_dir)?;

    // Получение списка крейтов воркспейса
    let crates = workspace_crates(&sh)?;
    eprintln!("Найдены крейты: {}", crates.join(", "));

    for crate_name in &crates {
        eprintln!("Генерация документации для {crate_name}...");

        // Генерация rustdoc JSON
        cmd!(
            sh,
            "cargo +nightly rustdoc -p {crate_name} -- -Z unstable-options --output-format json"
        )
        .run()?;

        let json_path = project.join(format!("target/doc/{crate_name}.json"));
        if !json_path.exists() {
            eprintln!("  Предупреждение: JSON не найден для {crate_name}");
            continue;
        }

        // Чтение и парсинг rustdoc JSON
        let json_content = fs::read_to_string(&json_path)
            .with_context(|| format!("не удалось прочитать {}", json_path.display()))?;
        let crate_data: rustdoc_types::Crate = serde_json::from_str(&json_content)
            .with_context(|| format!("не удалось распарсить JSON для {crate_name}"))?;

        // Конвертация в Markdown через API rustdoc-md
        let markdown = rustdoc_md::rustdoc_json_to_markdown(crate_data);

        // Пост-обработка: удаление строк перед первым "# " заголовком
        let markdown = strip_header_content(&markdown);

        // Запись результата
        let output_path = api_dir.join(format!("{crate_name}.md"));
        fs::write(&output_path, markdown)?;

        eprintln!("  -> {crate_name}.md сгенерирован");
    }

    eprintln!("API документация сгенерирована в docs/content/api/");
    Ok(())
}

/// Удалить строки перед первым markdown заголовком ("# ").
fn strip_header_content(content: &str) -> String {
    // Поиск первой строки, начинающейся с "# "
    if let Some(pos) = content.find("\n# ") {
        content[pos + 1..].to_string() // +1 чтобы пропустить перенос строки
    } else {
        content.to_string()
    }
}

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
}

/// Получить список крейтов воркспейса.
fn workspace_crates(sh: &Shell) -> Result<Vec<String>> {
    let output = cmd!(sh, "cargo metadata --no-deps --format-version 1").read()?;
    let metadata: CargoMetadata =
        serde_json::from_str(&output).context("не удалось распарсить cargo metadata")?;

    let crates: Vec<String> = metadata.packages.into_iter().map(|p| p.name).collect();
    Ok(crates)
}

/// Получить корневую директорию проекта.
///
/// Эта функция определяет корень проекта, находя родительскую директорию
/// директории манифеста текущего крейта.
///
/// # Возвращает
///
/// Абсолютный путь к корневой директории проекта.
///
/// # Ошибки
///
/// Возвращает ошибку, если:
/// - Переменная окружения CARGO_MANIFEST_DIR не установлена
/// - Директория манифеста не имеет родительской директории
fn project_root() -> Result<std::path::PathBuf> {
    Ok(std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)?
        .parent()
        .context("CARGO_MANIFEST_DIR не имеет родительской директории")?
        .to_path_buf())
}

/// Проверить наличие cargo-nextest и установить при необходимости.
///
/// Эта функция проверяет, установлен ли cargo-nextest в системе.
/// Если нет — автоматически устанавливает его через `cargo install`.
fn ensure_nextest(sh: &Shell) -> Result<()> {
    // Проверяем наличие nextest (ignore_status чтобы не паниковать если не найден)
    if cmd!(sh, "cargo nextest --version").ignore_status().run().is_ok() {
        return Ok(());
    }

    // Устанавливаем nextest
    eprintln!("cargo-nextest не найден, устанавливаю...");
    cmd!(sh, "cargo install cargo-nextest --locked").run()?;
    eprintln!("cargo-nextest успешно установлен");
    Ok(())
}
