//! Domínio do GranaFlow: tipos, dinheiro em centavos, datas e formatação pt-BR.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxKind {
    Income,
    Expense,
    Transfer,
}

impl TxKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TxKind::Income => "income",
            TxKind::Expense => "expense",
            TxKind::Transfer => "transfer",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "income" => TxKind::Income,
            "transfer" => TxKind::Transfer,
            _ => TxKind::Expense,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TxKind::Income => "Receita",
            TxKind::Expense => "Despesa",
            TxKind::Transfer => "Transferência",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountKind {
    Cash,
    Checking,
    Savings,
    Wallet,
    CreditCard,
}

impl AccountKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AccountKind::Cash => "cash",
            AccountKind::Checking => "checking",
            AccountKind::Savings => "savings",
            AccountKind::Wallet => "wallet",
            AccountKind::CreditCard => "credit_card",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "checking" => AccountKind::Checking,
            "savings" => AccountKind::Savings,
            "wallet" => AccountKind::Wallet,
            "credit_card" => AccountKind::CreditCard,
            _ => AccountKind::Cash,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AccountKind::Cash => "Dinheiro em espécie",
            AccountKind::Checking => "Conta corrente",
            AccountKind::Savings => "Conta poupança",
            AccountKind::Wallet => "Carteira digital",
            AccountKind::CreditCard => "Cartão de crédito",
        }
    }

    pub const ALL: [AccountKind; 5] = [
        AccountKind::Cash,
        AccountKind::Checking,
        AccountKind::Savings,
        AccountKind::Wallet,
        AccountKind::CreditCard,
    ];
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub kind: AccountKind,
    pub initial_cents: i64,
    pub archived: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub kind: TxKind,
    pub color: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub id: i64,
    pub kind: TxKind,
    pub date: NaiveDate,
    pub amount_cents: i64,
    pub account_id: i64,
    pub to_account_id: Option<i64>,
    pub category_id: Option<i64>,
    pub description: String,
    pub paid: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Budget {
    pub id: i64,
    pub month: String,
    pub category_id: i64,
    pub limit_cents: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Goal {
    pub id: i64,
    pub name: String,
    pub target_cents: i64,
    pub saved_cents: i64,
    pub deadline: Option<NaiveDate>,
}

/// "2026-11" para qualquer data do mês.
pub fn month_key(d: NaiveDate) -> String {
    format!("{:04}-{:02}", d.year(), d.month())
}

/// Primeiro dia do mês a partir de "2026-11".
pub fn month_start(key: &str) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(
        key.get(0..4)?.parse().ok()?,
        key.get(5..7)?.parse().ok()?,
        1,
    )
}

/// Desloca "2026-11" por n meses (-1 = mês anterior).
pub fn shift_month(key: &str, delta: i32) -> String {
    let Some(d) = month_start(key) else {
        return key.to_string();
    };
    let idx = d.year() * 12 + d.month0() as i32 + delta;
    format!("{:04}-{:02}", idx.div_euclid(12), idx.rem_euclid(12) + 1)
}

pub fn today() -> NaiveDate {
    chrono::Local::now().naive_local().date()
}

/// R$ 1.234,56 — negativo vira "-R$ 1.234,56".
pub fn fmt_brl(cents: i64) -> String {
    let neg = cents < 0;
    let v = cents.unsigned_abs();
    let reais = (v / 100).to_string();
    let mut grouped = String::with_capacity(reais.len() + reais.len() / 3);
    for (i, ch) in reais.chars().enumerate() {
        if i > 0 && (reais.len() - i) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    format!("{}R$ {},{:02}", if neg { "-" } else { "" }, grouped, v % 100)
}

/// Aceita "1.234,56", "1234,56", "1234.56", "R$ 1234". Nulo se inválido.
pub fn parse_brl(s: &str) -> Option<i64> {
    let raw: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, 'R' | '$'))
        .collect();
    if raw.is_empty() {
        return None;
    }
    let neg = raw.starts_with('-');
    let t = raw.trim_start_matches(['-', '+']);
    if t.is_empty() {
        return None;
    }
    let norm = if t.contains(',') {
        t.replace('.', "").replace(',', ".")
    } else {
        t.to_string()
    };
    let f: f64 = norm.parse().ok()?;
    if !f.is_finite() || f < 0.0 {
        return None;
    }
    Some((f * 100.0).round() as i64 * if neg { -1 } else { 1 })
}

/// Interpreta "2026-11-08" ou "08/11/2026".
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let t = s.trim();
    NaiveDate::parse_from_str(t, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(t, "%d/%m/%Y"))
        .ok()
}

pub fn fmt_date(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_roundtrip() {
        assert_eq!(fmt_brl(123456), "R$ 1.234,56");
        assert_eq!(fmt_brl(-5), "-R$ 0,05");
        assert_eq!(fmt_brl(0), "R$ 0,00");
        assert_eq!(parse_brl("1.234,56"), Some(123456));
        assert_eq!(parse_brl("1234.56"), Some(123456));
        assert_eq!(parse_brl("R$ 9"), Some(900));
        assert_eq!(parse_brl("-10,00"), Some(-1000));
        assert_eq!(parse_brl("abc"), None);
        assert_eq!(parse_brl(""), None);
    }

    #[test]
    fn month_shift() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
        assert_eq!(month_key(d), "2026-11");
        assert_eq!(shift_month("2026-11", -1), "2026-10");
        assert_eq!(shift_month("2026-01", -1), "2025-12");
        assert_eq!(shift_month("2026-12", 1), "2027-01");
        assert_eq!(month_start("2026-11").unwrap().day(), 1);
    }

    #[test]
    fn dates() {
        assert_eq!(parse_date("08/11/2026").unwrap().year(), 2026);
        assert_eq!(parse_date("2026-11-08").unwrap().day(), 8);
        assert!(parse_date("31/02/2026").is_none());
    }
}
