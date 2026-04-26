use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Допоміжна функція: запускає наш бінарник `snippets-app`
fn bin() -> Command {
    Command::cargo_bin("snippets-app").expect("binary snippets-app not found")
}

/// Тест 1: створити сніпет зі stdin і потім його прочитати
#[test]
fn create_and_read_snippet_from_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let store_path = temp.path().join("store.json");

    // 1) створюємо сніпет
    let mut cmd = bin();
    cmd.env("SNIPPETS_APP_STORE", &store_path)
        .arg("--name")
        .arg("hello_test")
        .write_stdin(r#"println!("Hello from test");"#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("OK:"));

    // 2) читаємо той самий сніпет
    let mut cmd = bin();
    cmd.env("SNIPPETS_APP_STORE", &store_path)
        .arg("--read")
        .arg("hello_test");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#"println!("Hello from test");"#));

    Ok(())
}

/// Тест 2: створити, видалити і переконатися, що читання падає з помилкою
#[test]
fn delete_snippet_removes_it() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let store_path = temp.path().join("store.json");

    // Створюємо сніпет
    let mut cmd = bin();
    cmd.env("SNIPPETS_APP_STORE", &store_path)
        .arg("--name")
        .arg("to_delete")
        .write_stdin("code to delete");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("OK:"));

    // Видаляємо
    let mut cmd = bin();
    cmd.env("SNIPPETS_APP_STORE", &store_path)
        .arg("--delete")
        .arg("to_delete");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("OK:"));

    // Спроба прочитати має завершитися з помилкою
    let mut cmd = bin();
    cmd.env("SNIPPETS_APP_STORE", &store_path)
        .arg("--read")
        .arg("to_delete");

    cmd.assert().failure().stderr(
        predicate::str::contains("Сніпет не знайдено").or(predicate::str::contains("not found")),
    );

    Ok(())
}

/// Тест 3: логування в файл через SNIPPETS_APP_LOG_LEVEL та SNIPPETS_APP_LOG_PATH
#[test]
fn logs_are_written_to_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let store_path = temp.path().join("store.json");
    let log_path = temp.path().join("snippets.log");

    // створюємо сніпет із логуванням у файл
    let mut cmd = bin();
    cmd.env("SNIPPETS_APP_STORE", &store_path)
        .env("SNIPPETS_APP_LOG_LEVEL", "info")
        .env("SNIPPETS_APP_LOG_PATH", &log_path)
        .arg("--name")
        .arg("log_test")
        .write_stdin("fn main() {}");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("OK:"));

    // перевіряємо, що файл логів створено і в ньому є щось
    assert!(log_path.exists(), "log file was not created");
    let content = fs::read_to_string(&log_path)?;
    assert!(
        content.contains("Saved snippet") || content.to_lowercase().contains("saved snippet"),
        "log file does not contain 'Saved snippet', content was:\n{}",
        content
    );

    Ok(())
}

/// Тест 4: некоректний URL у --download має давати помилку
#[test]
fn download_with_invalid_url_fails() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let store_path = temp.path().join("store.json");

    let mut cmd = bin();
    cmd.env("SNIPPETS_APP_STORE", &store_path)
        .arg("--name")
        .arg("bad_url")
        .arg("--download")
        .arg("not-a-valid-url");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Error").or(predicate::str::contains("error")));

    Ok(())
}
