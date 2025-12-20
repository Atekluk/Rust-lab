use anyhow::{anyhow, Context, Result}; // Імпортуємо необхідні елементи з anyhow
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    name: Option<String>,

    #[arg(long)]
    read: Option<String>,

    #[arg(long)]
    delete: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct Store {
    snippets: HashMap<String, String>,
}

impl Store {
    // Тепер повертає Result<Store> замість паніки
    fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let file = File::open(path)
                .with_context(|| format!("Не вдалося відкрити файл сховища: {}", path.display()))?;

            let store = serde_json::from_reader(file)
                .context("Не вдалося десеріалізувати (розібрати) файл сховища")?;

            Ok(store)
        } else {
            Ok(Store::default())
        }
    }

    // Тепер повертає Result<()>
    fn save(&self, path: &Path) -> Result<()> {
        let file = File::create(path)
            .context("Не вдалося створити файл сховища")?;

        serde_json::to_writer_pretty(file, self)
            .context("Не вдалося записати дані у файл")?;

        Ok(())
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let store_path = Path::new("snippets.json");

    // Завантажуємо сховище, прокидаючи помилку, якщо вона є
    let mut store = Store::load(store_path)?;

    if let Some(name) = args.name {
        let mut content = String::new();
        io::stdin()
            .read_to_string(&mut content)
            .context("Не вдалося прочитати дані з stdin")?; // Додаємо контекст помилки

        let content = content.trim().to_string();

        store.snippets.insert(name.clone(), content);
        store.save(store_path)?; // Зберігаємо зміни
        eprintln!("Сніпет '{}' успішно збережено.", name);

    } else if let Some(name) = args.read {
        match store.snippets.get(&name) {
            Some(content) => println!("{}", content),
            None => {
                // Повертаємо помилку замість exit(1), anyhow обробить це і виведе повідомлення
                return Err(anyhow!("Сніпет '{}' не знайдено.", name));
            }
        }

    } else if let Some(name) = args.delete {
        if store.snippets.remove(&name).is_some() {
            store.save(store_path)?;
            eprintln!("Сніпет '{}' видалено.", name);
        } else {
            return Err(anyhow!("Сніпет '{}' не знайдено.", name));
        }
    } else {
        use clap::CommandFactory;
        Args::command().print_help().context("Не вдалося вивести довідку")?;
    }

    Ok(())
}