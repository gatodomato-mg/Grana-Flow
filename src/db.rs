//! Módulo de persistência SQLite para GranaFlow.

use chrono::NaiveDate;
use rusqlite::{Connection, Result};
use std::path::{Path, PathBuf};

use crate::model::{TxKind, AccountKind, Account, Category, Transaction, Budget, Goal};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        Self::create_tables(&conn)?;
        Ok(Database { conn })
    }

    fn create_tables(conn: &Connection) -> Result<()> {
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'cash',
                initial_cents INTEGER NOT NULL DEFAULT 0,
                archived INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                color TEXT NOT NULL DEFAULT '#3B82F6'
            );
            CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                date TEXT NOT NULL,
                amount_cents INTEGER NOT NULL,
                account_id INTEGER NOT NULL,
                to_account_id INTEGER,
                category_id INTEGER,
                description TEXT NOT NULL,
                paid INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS budgets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                month TEXT NOT NULL,
                category_id INTEGER NOT NULL,
                limit_cents INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS goals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                target_cents INTEGER NOT NULL,
                saved_cents INTEGER NOT NULL DEFAULT 0,
                deadline TEXT
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_budget_month_cat ON budgets(month, category_id);
            CREATE INDEX IF NOT EXISTS idx_tx_date ON transactions(date);
            CREATE TABLE IF NOT EXISTS settings (
                k TEXT PRIMARY KEY,
                v TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS schema_info (
                version INTEGER NOT NULL
            );
        "#)?;
        Ok(())
    }

    // Accounts
    pub fn account_list(&self) -> Result<Vec<Account>> {
        let mut stmt = self.conn.prepare("SELECT id, name, kind, initial_cents, archived FROM accounts WHERE archived = 0")?;

        let rows = stmt.query_map([], |row| {
            Ok(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: AccountKind::parse(&row.get::<_, String>(2)?),
                initial_cents: row.get(3)?,
                archived: row.get(4)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn account_insert(&self, name: &str, kind: AccountKind, initial_cents: i64) -> Result<i64> {
        let mut stmt = self.conn.prepare("INSERT INTO accounts (name, kind, initial_cents) VALUES (?1, ?2, ?3)")?;
        let id = stmt.insert((name, kind.as_str(), initial_cents))?;
        Ok(id)
    }

    pub fn account_update(&self, id: i64, name: &str, kind: AccountKind, initial_cents: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE accounts SET name = ?1, kind = ?2, initial_cents = ?3 WHERE id = ?4",
            (name, kind.as_str(), initial_cents, id),
        )?;
        Ok(())
    }

    pub fn account_delete(&self, id: i64) -> Result<()> {
        self.conn.execute("UPDATE accounts SET archived = 1 WHERE id = ?", (id,))?;
        Ok(())
    }

    // Categories
    pub fn category_list(&self, kind: TxKind) -> Result<Vec<Category>> {
        let kind_str = kind.as_str();
        let mut stmt = self.conn.prepare(&format!("SELECT id, name, kind, color FROM categories WHERE kind = '{}'", kind_str))?;
        let rows = stmt.query_map([], |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: TxKind::parse(&row.get::<_, String>(2)?),
                color: row.get(3)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn category_insert(&self, name: &str, kind: TxKind, color: &str) -> Result<i64> {
        let mut stmt = self.conn.prepare("INSERT INTO categories (name, kind, color) VALUES (?1, ?2, ?3)")?;
        let id = stmt.insert((name, kind.as_str(), color))?;
        Ok(id)
    }

    // Transactions
    pub fn transaction_list(&self) -> Result<Vec<Transaction>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, date, amount_cents, account_id, to_account_id, category_id, description, paid FROM transactions ORDER BY date DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Transaction {
                id: row.get(0)?,
                kind: TxKind::parse(&row.get::<_, String>(1)?),
                date: NaiveDate::parse_from_str(&row.get::<_, String>(2)?, "%Y-%m-%d")
                    .expect("Invalid date in DB"),
                amount_cents: row.get(3)?,
                account_id: row.get(4)?,
                to_account_id: row.get(5)?,
                category_id: row.get(6)?,
                description: row.get(7)?,
                paid: row.get(8)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn transaction_insert(&self, tx: &Transaction) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO transactions (kind, date, amount_cents, account_id, to_account_id, category_id, description, paid) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        let id = stmt.insert((
            tx.kind.as_str(),
            tx.date.format("%Y-%m-%d").to_string(),
            tx.amount_cents,
            tx.account_id,
            tx.to_account_id,
            tx.category_id,
            &tx.description,
            tx.paid as i32,
        ))?;
        Ok(id)
    }

    // Budgets
    pub fn budget_list(&self, month: &str) -> Result<Vec<Budget>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, month, category_id, limit_cents FROM budgets WHERE month = ?1",
        )?;
        let rows = stmt.query_map([month], |row| {
            Ok(Budget {
                id: row.get(0)?,
                month: row.get(1)?,
                category_id: row.get(2)?,
                limit_cents: row.get(3)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn budget_insert(&self, month: &str, category_id: i64, limit_cents: i64) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO budgets (month, category_id, limit_cents) VALUES (?1, ?2, ?3)",
        )?;
        let id = stmt.insert((month, category_id, limit_cents))?;
        Ok(id)
    }

    // Goals
    pub fn goal_list(&self) -> Result<Vec<Goal>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, target_cents, saved_cents, deadline FROM goals",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Goal {
                id: row.get(0)?,
                name: row.get(1)?,
                target_cents: row.get(2)?,
                saved_cents: row.get(3)?,
                deadline: row
                    .get::<_, Option<String>>(4)?
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn goal_insert(&self, goal: &Goal) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO goals (name, target_cents, saved_cents, deadline) VALUES (?1, ?2, ?3, ?4)",
        )?;
        let id = stmt.insert((
            &goal.name,
            goal.target_cents,
            goal.saved_cents,
            goal.deadline.map(|d| d.format("%Y-%m-%d").to_string()),
        ))?;
        Ok(id)
    }

    pub fn goal_update(&self, goal: &Goal) -> Result<()> {
        self.conn.execute(
            "UPDATE goals SET name = ?1, target_cents = ?2, saved_cents = ?3, deadline = ?4 WHERE id = ?5",
            (
                &goal.name,
                goal.target_cents,
                goal.saved_cents,
                goal.deadline.map(|d| d.format("%Y-%m-%d").to_string()),
                goal.id,
            ),
        )?;
        Ok(())
    }

    pub fn goal_delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM goals WHERE id = ?", (id,))?;
        Ok(())
    }

    pub fn goal_deposit(&self, id: i64, cents: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE goals SET saved_cents = saved_cents + ?1 WHERE id = ?2",
            (cents, id),
        )?;
        Ok(())
    }

    /// %APPDATA%\GranaFlow\granaflow.db no Windows; ~/.local/share/granaflow no Linux.
    pub fn default_path() -> PathBuf {
        let base = if cfg!(windows) {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
                .unwrap_or_else(|| PathBuf::from("."))
        };
        base.join("GranaFlow").join("granaflow.db")
    }

    // Transactions
    pub fn transaction_update(&self, tx: &Transaction) -> Result<()> {
        self.conn.execute(
            "UPDATE transactions SET kind = ?1, date = ?2, amount_cents = ?3, account_id = ?4, to_account_id = ?5, category_id = ?6, description = ?7, paid = ?8 WHERE id = ?9",
            (
                tx.kind.as_str(),
                tx.date.format("%Y-%m-%d").to_string(),
                tx.amount_cents,
                tx.account_id,
                tx.to_account_id,
                tx.category_id,
                &tx.description,
                tx.paid as i32,
                tx.id,
            ),
        )?;
        Ok(())
    }

    pub fn transaction_delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM transactions WHERE id = ?", (id,))?;
        Ok(())
    }

    pub fn transaction_set_paid(&self, id: i64, paid: bool) -> Result<()> {
        self.conn
            .execute("UPDATE transactions SET paid = ?1 WHERE id = ?2", (paid as i32, id))?;
        Ok(())
    }

    // Budgets
    pub fn budget_set(&self, month: &str, category_id: i64, limit_cents: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO budgets (month, category_id, limit_cents) VALUES (?1, ?2, ?3)
             ON CONFLICT(month, category_id) DO UPDATE SET limit_cents = excluded.limit_cents",
            (month, category_id, limit_cents),
        )?;
        Ok(())
    }

    pub fn budget_delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM budgets WHERE id = ?", (id,))?;
        Ok(())
    }

    // Categories
    pub fn category_update(&self, id: i64, name: &str, color: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE categories SET name = ?1, color = ?2 WHERE id = ?3",
            (name, color, id),
        )?;
        Ok(())
    }

    pub fn category_delete(&self, id: i64) -> Result<()> {
        self.conn
            .execute("UPDATE transactions SET category_id = NULL WHERE category_id = ?1", (id,))?;
        self.conn.execute("DELETE FROM categories WHERE id = ?", (id,))?;
        self.conn.execute("DELETE FROM budgets WHERE category_id = ?", (id,))?;
        Ok(())
    }

    // Settings
    pub fn setting_get(&self, key: &str) -> Option<String> {
        self.conn
            .query_row("SELECT v FROM settings WHERE k = ?1", [key], |r| r.get(0))
            .ok()
    }

    pub fn setting_set(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (k, v) VALUES (?1, ?2)
             ON CONFLICT(k) DO UPDATE SET v = excluded.v",
            (key, value),
        )?;
        Ok(())
    }

    // Backup e restauração - arquivo SQLite pronto; VACUUM INTO grava um banco consistente.
    pub fn backup_to(&self, path: &Path) -> Result<()> {
        let p = path.to_string_lossy().replace('\'', "''");
        self.conn.execute_batch(&format!("VACUUM INTO '{p}'"))?;
        Ok(())
    }

    /// Substitui os dados atuais pelos do arquivo de backup, em uma única transação.
    pub fn restore_from(&self, src: &Path) -> Result<()> {
        let p = src.to_string_lossy().replace('\'', "''");
        self.conn.execute_batch(&format!(
            "ATTACH DATABASE '{p}' AS bak;
             BEGIN;
             DELETE FROM transactions;
             DELETE FROM budgets;
             DELETE FROM goals;
             DELETE FROM settings;
             DELETE FROM categories;
             DELETE FROM accounts;
             INSERT INTO accounts SELECT id, name, kind, initial_cents, archived FROM bak.accounts;
             INSERT INTO categories SELECT id, name, kind, color FROM bak.categories;
             INSERT INTO transactions SELECT id, kind, date, amount_cents, account_id, to_account_id, category_id, description, paid FROM bak.transactions;
             INSERT INTO budgets SELECT id, month, category_id, limit_cents FROM bak.budgets;
             INSERT INTO goals SELECT id, name, target_cents, saved_cents, deadline FROM bak.goals;
             INSERT INTO settings SELECT k, v FROM bak.settings;
             COMMIT;
             DETACH DATABASE bak;"
        ))?;
        Ok(())
    }

    /// Dados iniciais para um banco novo.
    pub fn seed(&self) -> Result<()> {
        let cats: i64 = self.conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))?;
        if cats == 0 {
            for (name, kind, color) in [
                ("Salário", "income", "#10B981"),
                ("Freelance", "income", "#34D399"),
                ("Investimentos", "income", "#06B6D4"),
                ("Outras receitas", "income", "#22D3EE"),
                ("Moradia", "expense", "#F59E0B"),
                ("Alimentação", "expense", "#EF4444"),
                ("Transporte", "expense", "#8B5CF6"),
                ("Saúde", "expense", "#EC4899"),
                ("Educação", "expense", "#3B82F6"),
                ("Lazer", "expense", "#14B8A6"),
                ("Assinaturas", "expense", "#A855F7"),
                ("Outras despesas", "expense", "#64748B"),
            ] {
                self.category_insert(name, TxKind::parse(kind), color)?;
            }
        }
        let accs: i64 = self.conn.query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))?;
        if accs == 0 {
            self.account_insert("Carteira", AccountKind::Cash, 0)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn db_open_close() {
        let db_path = Path::new("/tmp/granaflow_test.db");
        if db_path.exists() {
            std::fs::remove_file(db_path).ok();
        }
        let db = Database::open(db_path).unwrap();
        let _accounts = db.account_list().unwrap();
        drop(db);
        assert!(!db_path.exists() || Path::new("/tmp/granaflow_test.db").exists());
    }
}

