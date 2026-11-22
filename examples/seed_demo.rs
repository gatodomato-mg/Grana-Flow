//! Gera um banco de demonstração para explorar a interface.
//! Uso: cargo run --example seed_demo
//! Depois: set GRANAFLOW_DB=demo.db && cargo run

use chrono::NaiveDate;
use granaflow::db::Database;
use granaflow::model::*;
use std::path::PathBuf;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn main() {
    let path = std::env::var_os("GRANAFLOW_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("demo.db"));
    let _ = std::fs::remove_file(&path);
    let db = Database::open(&path).expect("abrir demo.db");
    db.seed().expect("seed");

    let cash = db.account_insert("Carteira", AccountKind::Cash, 0).unwrap();
    let checking = db
        .account_insert("Conta corrente", AccountKind::Checking, 150_000)
        .unwrap();
    let savings = db
        .account_insert("Poupança", AccountKind::Savings, 800_000)
        .unwrap();
    let card = db
        .account_insert("Cartão de crédito", AccountKind::CreditCard, 0)
        .unwrap();

    let cats = |kind: TxKind| db.category_list(kind).unwrap();
    let income = cats(TxKind::Income);
    let expense = cats(TxKind::Expense);
    let cat = |list: &[Category], name: &str| list.iter().find(|c| c.name == name).map(|c| c.id);

    let mut add = |kind: TxKind,
                   date: NaiveDate,
                   cents: i64,
                   account: i64,
                   to: Option<i64>,
                   category: Option<i64>,
                   desc: &str,
                   paid: bool| {
        let tx = Transaction {
            id: 0,
            kind,
            date,
            amount_cents: cents,
            account_id: account,
            to_account_id: to,
            category_id: category,
            description: desc.to_string(),
            paid,
        };
        db.transaction_insert(&tx).unwrap();
    };

    let salary = cat(&income, "Salário");
    let freelance = cat(&income, "Freelance");
    let other_in = cat(&income, "Outras receitas");
    let housing = cat(&expense, "Moradia");
    let food = cat(&expense, "Alimentação");
    let transport = cat(&expense, "Transporte");
    let health = cat(&expense, "Saúde");
    let leisure = cat(&expense, "Lazer");
    let subs = cat(&expense, "Assinaturas");
    let other_out = cat(&expense, "Outras despesas");

    for (m, day) in [(8, 5), (9, 5), (10, 5), (11, 5)] {
        add(TxKind::Income, d(2026, m, day), 450_000, checking, None, salary, "Salário", true);
        add(TxKind::Expense, d(2026, m, 10), 150_000, checking, None, housing, "Aluguel", true);
        add(TxKind::Expense, d(2026, m, 12), 48_000, card, None, food, "Mercado", true);
        add(TxKind::Expense, d(2026, m, 18), 22_000, checking, None, transport, "Combustível", true);
        add(TxKind::Expense, d(2026, m, 20), 4_990, card, None, subs, "Assinaturas", true);
    }
    add(TxKind::Income, d(2026, 10, 20), 120_000, checking, None, freelance, "Freelance de design", true);
    add(TxKind::Income, d(2026, 11, 8), 5_000, cash, None, other_in, "Reembolso", false);
    add(TxKind::Expense, d(2026, 9, 15), 25_000, checking, None, health, "Consulta médica", true);
    add(TxKind::Expense, d(2026, 10, 3), 32_000, card, None, leisure, "Show", true);
    add(TxKind::Expense, d(2026, 11, 2), 89_990, card, None, other_out, "Notebook", true);
    add(TxKind::Expense, d(2026, 11, 20), 18_000, checking, None, housing, "Conta de luz", false);
    add(TxKind::Transfer, d(2026, 11, 1), 100_000, checking, Some(savings), None, "Reserva do mês", true);
    let _ = cash;

    for (name, limit) in [
        ("Moradia", 160_000),
        ("Alimentação", 60_000),
        ("Transporte", 25_000),
        ("Lazer", 20_000),
        ("Assinaturas", 6_000),
    ] {
        if let Some(id) = cat(&expense, name) {
            db.budget_set("2026-11", id, limit).unwrap();
        }
    }

    db.goal_insert(&Goal {
        id: 0,
        name: "Reserva de emergência".into(),
        target_cents: 1_800_000,
        saved_cents: 450_000,
        deadline: Some(d(2027, 12, 31)),
    })
    .unwrap();
    db.goal_insert(&Goal {
        id: 0,
        name: "Viagem Nordeste".into(),
        target_cents: 800_000,
        saved_cents: 120_000,
        deadline: Some(d(2027, 7, 1)),
    })
    .unwrap();
    db.goal_insert(&Goal {
        id: 0,
        name: "Notebook novo".into(),
        target_cents: 900_000,
        saved_cents: 89_990,
        deadline: None,
    })
    .unwrap();

    println!("demo criado em {}", path.display());
}
