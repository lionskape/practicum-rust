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
/// 1. Запускает тесты и сохраняет результаты в документации
/// 2. Генерирует rustdoc JSON для всех крейтов воркспейса
/// 3. Конвертирует JSON в Markdown через rustdoc-md
/// 4. Устанавливает зависимости Nextra
/// 5. Собирает статический сайт документации
///
/// Итоговая документация будет доступна в `docs/out/`.
fn docs_build() -> Result<()> {
    let sh = Shell::new()?;
    let docs_dir = project_root()?.join("docs");

    // Запуск CI проверок и сохранение результатов
    docs_ci(&sh)?;

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

/// Запустить CI проверки и сохранить результаты в документации.
///
/// Эта функция создаёт папку `docs/content/ci/` и сохраняет результаты:
/// - `fmt.md` — проверка форматирования
/// - `clippy.md` — линтер Clippy
/// - `tests.md` — unit-тесты (nextest)
/// - `doctests.md` — doc-тесты
fn docs_ci(sh: &Shell) -> Result<()> {
    let project = project_root()?;
    let ci_dir = project.join("docs/content/ci");

    // Создание директории ci
    fs::create_dir_all(&ci_dir)?;

    eprintln!("Запуск CI проверок для документации...");

    // Запуск всех проверок
    let fmt_result = run_ci_check(sh, "fmt", "cargo +nightly fmt --all -- --check")?;
    let clippy_result =
        run_ci_check(sh, "clippy", "cargo +nightly clippy --workspace -- -D warnings")?;

    ensure_nextest(sh)?;
    let tests_result = run_ci_check(sh, "tests", "cargo nextest run --workspace --color=never")?;
    let doctests_result =
        run_ci_check(sh, "doctests", "cargo +nightly test --workspace --doc --color=never")?;

    let timestamp = chrono_lite_now();

    // Сохранение результатов в отдельные файлы
    write_ci_result(&ci_dir, "fmt", "Форматирование (rustfmt)", &fmt_result, &timestamp)?;
    write_ci_result(&ci_dir, "clippy", "Линтер Clippy", &clippy_result, &timestamp)?;
    write_ci_result(&ci_dir, "tests", "Unit-тесты (nextest)", &tests_result, &timestamp)?;
    write_ci_result(&ci_dir, "doctests", "Doc-тесты", &doctests_result, &timestamp)?;

    // Создание индексной страницы CI
    let all_passed = fmt_result.success
        && clippy_result.success
        && tests_result.success
        && doctests_result.success;
    write_ci_index(
        &ci_dir,
        &timestamp,
        all_passed,
        &[
            ("fmt", "Форматирование", fmt_result.success),
            ("clippy", "Clippy", clippy_result.success),
            ("tests", "Unit-тесты", tests_result.success),
            ("doctests", "Doc-тесты", doctests_result.success),
        ],
    )?;

    let status = if all_passed {
        "✅ Все проверки пройдены"
    } else {
        "❌ Есть ошибки"
    };
    eprintln!("  -> CI результаты сгенерированы ({status})");

    Ok(())
}

/// Результат CI проверки.
struct CiCheckResult {
    success: bool,
    stdout: String,
    stderr: String,
}

/// Запустить CI проверку и вернуть результат.
fn run_ci_check(_sh: &Shell, name: &str, command: &str) -> Result<CiCheckResult> {
    eprintln!("  Запуск {name}...");

    // Разбиваем команду на части
    let parts: Vec<&str> = command.split_whitespace().collect();
    let (program, args) = parts.split_first().context("пустая команда")?;

    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("не удалось запустить {command}"))?;

    Ok(CiCheckResult {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Записать результат CI проверки в markdown файл.
fn write_ci_result(
    ci_dir: &std::path::Path,
    filename: &str,
    title: &str,
    result: &CiCheckResult,
    timestamp: &str,
) -> Result<()> {
    let status_emoji = if result.success { "✅" } else { "❌" };
    let status_text = if result.success { "Успешно" } else { "Ошибки" };

    let mut content = String::new();
    content.push_str(&format!("# {title}\n\n"));
    content.push_str(&format!("> **Статус:** {status_emoji} {status_text}\n"));
    content.push_str(&format!("> **Дата:** {timestamp}\n\n"));

    // Вывод команды
    if !result.stderr.is_empty() || !result.stdout.is_empty() {
        content.push_str("## Вывод\n\n```\n");
        if !result.stderr.is_empty() {
            content.push_str(&result.stderr);
        }
        if !result.stdout.is_empty() {
            content.push_str(&result.stdout);
        }
        content.push_str("```\n");
    } else {
        content.push_str("*Нет вывода*\n");
    }

    let path = ci_dir.join(format!("{filename}.md"));
    fs::write(&path, content)?;

    let status = if result.success { "✓" } else { "✗" };
    eprintln!("    [{status}] {filename}.md");

    Ok(())
}

/// Записать индексную страницу CI.
fn write_ci_index(
    ci_dir: &std::path::Path,
    timestamp: &str,
    all_passed: bool,
    checks: &[(&str, &str, bool)],
) -> Result<()> {
    let status_emoji = if all_passed { "✅" } else { "❌" };
    let status_text = if all_passed {
        "Все проверки пройдены"
    } else {
        "Есть ошибки"
    };

    let mut content = String::new();
    content.push_str("# Результаты CI\n\n");
    content.push_str(&format!("> **Статус:** {status_emoji} {status_text}\n"));
    content.push_str(&format!("> **Дата:** {timestamp}\n\n"));

    content.push_str("## Проверки\n\n");
    content.push_str("| Проверка | Статус |\n");
    content.push_str("|----------|--------|\n");

    for (filename, title, success) in checks {
        let emoji = if *success { "✅" } else { "❌" };
        content.push_str(&format!("| [{title}](./{filename}) | {emoji} |\n"));
    }

    let path = ci_dir.join("index.md");
    fs::write(&path, content)?;

    Ok(())
}

/// Получить текущую дату и время в формате ISO 8601.
///
/// Простая реализация без внешних зависимостей.
fn chrono_lite_now() -> String {
    use std::time::SystemTime;

    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();

    // Конвертация в компоненты даты (упрощённо, UTC)
    let secs = now.as_secs();
    let days = secs / 86400;
    let time_secs = secs % 86400;

    // Вычисление года, месяца, дня (упрощённый алгоритм для 2000-2099)
    let mut year = 1970;
    let mut remaining_days = days as i64;

    while remaining_days >= days_in_year(year) {
        remaining_days -= days_in_year(year);
        year += 1;
    }

    let mut month = 1;
    while remaining_days >= days_in_month(year, month) {
        remaining_days -= days_in_month(year, month);
        month += 1;
    }

    let day = remaining_days + 1;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    format!("{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}:{seconds:02} UTC")
}

fn days_in_year(year: i64) -> i64 {
    if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 366 } else { 365 }
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
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
    // Проверяем наличие nextest (quiet чтобы не выводить в консоль)
    // Без ignore_status(): если команда завершится с ошибкой, run() вернёт Err
    if cmd!(sh, "cargo nextest --version").quiet().run().is_ok() {
        return Ok(());
    }

    // Устанавливаем nextest
    eprintln!("cargo-nextest не найден, устанавливаю...");
    cmd!(sh, "cargo install cargo-nextest --locked").run()?;
    eprintln!("cargo-nextest успешно установлен");
    Ok(())
}
