//! Regras financeiras do GranaFlow. Funções puras sobre slices: fácil de testar.
//! Convenções:
//! - transferência nunca é receita nem despesa; só muda o saldo das contas envolvidas.
//! - saldo de conta usa apenas lançamentos pagos (realizado).
//! - orçamento e relatórios mensais usam todos os lançamentos do mês (regime de competência).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::model::{month_key, shift_month, Account, Transaction, TxKind};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MonthSummary {
    pub income_paid: i64,
    pub income_pending: i64,
    pub expense_paid: i64,
    pub expense_pending: i64,
}

impl MonthSummary {
    pub fn result(&self) -> i64 {
        self.income_paid - self.expense_paid
    }

    pub fn to_pay(&self) -> i64 {
        self.expense_pending
    }

    pub fn to_receive(&self) -> i64 {
        self.income_pending
    }
}

pub fn month_summary(txs: &[Transaction], month: &str) -> MonthSummary {
    let mut s = MonthSummary::default();
    for t in txs.iter().filter(|t| month_key(t.date) == month) {
        match (t.kind, t.paid) {
            (TxKind::Income, true) => s.income_paid += t.amount_cents,
            (TxKind::Income, false) => s.income_pending += t.amount_cents,
            (TxKind::Expense, true) => s.expense_paid += t.amount_cents,
            (TxKind::Expense, false) => s.expense_pending += t.amount_cents,
            (TxKind::Transfer, _) => {}
        }
    }
    s
}

/// Saldo realizado de uma conta (transferências entram e saem).
pub fn account_balance(account: &Account, txs: &[Transaction]) -> i64 {
    let mut balance = account.initial_cents;
    for t in txs.iter().filter(|t| t.paid) {
        match t.kind {
            TxKind::Income if t.account_id == account.id => balance += t.amount_cents,
            TxKind::Expense if t.account_id == account.id => balance -= t.amount_cents,
            TxKind::Transfer => {
                if t.account_id == account.id {
                    balance -= t.amount_cents;
                }
                if t.to_account_id == Some(account.id) {
                    balance += t.amount_cents;
                }
            }
            _ => {}
        }
    }
    balance
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetWorth {
    pub assets: i64,
    pub debts: i64,
    pub net: i64,
}

pub fn net_worth(accounts: &[Account], txs: &[Transaction]) -> NetWorth {
    let mut w = NetWorth::default();
    for a in accounts {
        let b = account_balance(a, txs);
        if b >= 0 {
            w.assets += b;
        } else {
            w.debts += -b;
        }
    }
    w.net = w.assets - w.debts;
    w
}

/// Gasto total por categoria no mês (despesas, pagas ou não).
pub fn category_spend(txs: &[Transaction], month: &str) -> Vec<(i64, i64)> {
    let mut out: Vec<(i64, i64)> = Vec::new();
    for t in txs.iter().filter(|t| {
        t.kind == TxKind::Expense && month_key(t.date) == month && t.category_id.is_some()
    }) {
        let cat = t.category_id.unwrap();
        match out.iter_mut().find(|(c, _)| *c == cat) {
            Some((_, v)) => *v += t.amount_cents,
            None => out.push((cat, t.amount_cents)),
        }
    }
    out.sort_by(|a, b| b.1.cmp(&a.1));
    out
}

#[derive(Clone, Copy, Debug)]
pub struct BudgetLine {
    pub category_id: i64,
    pub limit_cents: i64,
    pub spent_cents: i64,
}

impl BudgetLine {
    pub fn remaining(&self) -> i64 {
        self.limit_cents - self.spent_cents
    }

    /// 0.0..=1.0+ (acima de 1 = estourou).
    pub fn ratio(&self) -> f32 {
        if self.limit_cents <= 0 {
            0.0
        } else {
            self.spent_cents as f32 / self.limit_cents as f32
        }
    }

    pub fn over(&self) -> bool {
        self.spent_cents > self.limit_cents
    }
}

pub fn budget_status(
    limits: &[(i64, i64)],
    txs: &[Transaction],
    month: &str,
) -> Vec<BudgetLine> {
    let spend = category_spend(txs, month);
    let mut out: Vec<BudgetLine> = limits
        .iter()
        .map(|&(category_id, limit_cents)| BudgetLine {
            category_id,
            limit_cents,
            spent_cents: spend
                .iter()
                .find(|(c, _)| *c == category_id)
                .map(|(_, v)| *v)
                .unwrap_or(0),
        })
        .collect();
    out.sort_by(|a, b| b.ratio().partial_cmp(&a.ratio()).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// Saldo acumulado ao fim de cada mês da janela (n meses até `end_month`).
pub fn evolution(txs: &[Transaction], end_month: &str, n: u32) -> Vec<(String, i64)> {
    let months: Vec<String> = (0..n)
        .rev()
        .map(|i| shift_month(end_month, -(i as i32)))
        .collect();
    let mut out = Vec::with_capacity(months.len());
    let mut running: i64 = 0;
    for m in &months {
        running += month_summary(txs, m).result();
        out.push((m.clone(), running));
    }
    out
}

/// Média do resultado mensal dos últimos `n` meses completos antes de `month`.
pub fn avg_monthly_result(txs: &[Transaction], month: &str, n: u32) -> i64 {
    if n == 0 {
        return 0;
    }
    let total: i64 = (1..=n)
        .map(|i| month_summary(txs, &shift_month(month, -(i as i32))).result())
        .sum();
    total / n as i64
}

/// Projeta saldo mês a mês. Premissa explícita: resultado mensal médio constante.
pub fn forecast(balance: i64, monthly_result: i64, months: u32) -> Vec<i64> {
    let mut out = Vec::with_capacity(months as usize);
    let mut cur = balance;
    for _ in 0..months {
        cur += monthly_result;
        out.push(cur);
    }
    out
}

/// Meses necessários para atingir a meta (arredonda para cima). None se não houver aporte.
pub fn months_to_goal(target_cents: i64, saved_cents: i64, monthly_cents: i64) -> Option<u32> {
    if saved_cents >= target_cents {
        return Some(0);
    }
    if monthly_cents <= 0 {
        return None;
    }
    let remaining = target_cents - saved_cents;
    Some(((remaining + monthly_cents - 1) / monthly_cents) as u32)
}

pub fn goal_eta(
    target_cents: i64,
    saved_cents: i64,
    monthly_cents: i64,
    from: NaiveDate,
) -> Option<NaiveDate> {
    let months = months_to_goal(target_cents, saved_cents, monthly_cents)?;
    let key = shift_month(&month_key(from), months as i32);
    crate::model::month_start(&key)
}

/// Aporte mensal necessário para chegar na meta até o prazo.
pub fn required_monthly(target_cents: i64, saved_cents: i64, months: i64) -> i64 {
    if months <= 0 {
        return target_cents - saved_cents;
    }
    let remaining = (target_cents - saved_cents).max(0);
    (remaining + months - 1) / months
}

/// Reserva de emergência sugerida: 6x a média de despesas pagas dos últimos 6 meses.
pub fn emergency_reserve(txs: &[Transaction], month: &str, months_of_expenses: i64) -> i64 {
    let total: i64 = (1..=6)
        .map(|i| month_summary(txs, &shift_month(month, -i)).expense_paid)
        .sum();
    (total / 6) * months_of_expenses.max(0)
}

/// CSV com cabeçalho; datas ISO, valores em centavos para não perder precisão.
pub fn transactions_csv(
    txs: &[Transaction],
    account_name: &dyn Fn(i64) -> String,
    category_name: &dyn Fn(i64) -> String,
) -> String {
    let mut s = String::from("data,tipo,descricao,categoria,conta,valor_centavos,pago\n");
    for t in txs {
        let cat = t.category_id.map(|c| category_name(c)).unwrap_or_default();
        s.push_str(&format!(
            "{},{},\"{}\",\"{}\",\"{}\",{},{}\n",
            t.date.format("%Y-%m-%d"),
            t.kind.as_str(),
            t.description.replace('"', "\"\""),
            cat.replace('"', "\"\""),
            account_name(t.account_id).replace('"', "\"\""),
            t.amount_cents,
            if t.paid { 1 } else { 0 }
        ));
    }
    s
}

#[derive(Serialize, Deserialize, Default)]
pub struct Snapshot {
    pub version: u32,
    pub accounts: Vec<Account>,
    pub categories: Vec<crate::model::Category>,
    pub transactions: Vec<Transaction>,
    pub budgets: Vec<crate::model::Budget>,
    pub goals: Vec<GoalSnapshot>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct GoalSnapshot {
    pub name: String,
    pub target_cents: i64,
    pub saved_cents: i64,
    pub deadline: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Account, AccountKind, Transaction, TxKind};
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn tx(kind: TxKind, date: NaiveDate, cents: i64, account: i64, paid: bool) -> Transaction {
        Transaction {
            id: 0,
            kind,
            date,
            amount_cents: cents,
            account_id: account,
            to_account_id: None,
            category_id: Some(1),
            description: String::new(),
            paid,
        }
    }

    fn acc(id: i64, initial: i64) -> Account {
        Account {
            id,
            name: format!("Conta {id}"),
            kind: AccountKind::Checking,
            initial_cents: initial,
            archived: false,
        }
    }

    #[test]
    fn summary_ignores_transfers_and_separates_pending() {
        let mut transfer = tx(TxKind::Transfer, d(2026, 11, 2), 5000, 1, true);
        transfer.to_account_id = Some(2);
        let txs = vec![
            tx(TxKind::Income, d(2026, 11, 1), 100_00, 1, true),
            tx(TxKind::Income, d(2026, 11, 3), 20_00, 1, false),
            tx(TxKind::Expense, d(2026, 11, 4), 30_00, 1, true),
            tx(TxKind::Expense, d(2026, 11, 5), 5_00, 1, false),
            transfer,
            tx(TxKind::Income, d(2026, 10, 1), 999_00, 1, true),
        ];
        let s = month_summary(&txs, "2026-11");
        assert_eq!(s.income_paid, 100_00);
        assert_eq!(s.income_pending, 20_00);
        assert_eq!(s.expense_paid, 30_00);
        assert_eq!(s.expense_pending, 5_00);
        assert_eq!(s.result(), 70_00);
    }

    #[test]
    fn transfer_moves_money_without_changing_net() {
        let mut t = tx(TxKind::Transfer, d(2026, 11, 2), 50_00, 1, true);
        t.to_account_id = Some(2);
        let accounts = vec![acc(1, 100_00), acc(2, 0)];
        let txs = vec![t];
        assert_eq!(account_balance(&accounts[0], &txs), 50_00);
        assert_eq!(account_balance(&accounts[1], &txs), 50_00);
        assert_eq!(net_worth(&accounts, &txs).net, 100_00);
    }

    #[test]
    fn credit_card_debt_counts_negative() {
        let accounts = vec![acc(1, 100_00), acc(2, 0)];
        let txs = vec![tx(TxKind::Expense, d(2026, 11, 2), 30_00, 2, true)];
        let w = net_worth(&accounts, &txs);
        assert_eq!(w.assets, 100_00);
        assert_eq!(w.debts, 30_00);
        assert_eq!(w.net, 70_00);
    }

    #[test]
    fn budget_flags_overspend() {
        let txs = vec![
            tx(TxKind::Expense, d(2026, 11, 2), 120_00, 1, true),
            tx(TxKind::Expense, d(2026, 11, 3), 40_00, 1, false),
        ];
        let lines = budget_status(&[(1, 100_00), (2, 50_00)], &txs, "2026-11");
        assert_eq!(lines[0].category_id, 1);
        assert_eq!(lines[0].spent_cents, 160_00);
        assert!(lines[0].over());
        assert_eq!(lines[1].spent_cents, 0);
        assert!(!lines[1].over());
    }

    #[test]
    fn evolution_is_cumulative() {
        let txs = vec![
            tx(TxKind::Income, d(2026, 09, 1), 100_00, 1, true),
            tx(TxKind::Expense, d(2026, 10, 1), 30_00, 1, true),
            tx(TxKind::Income, d(2026, 11, 1), 50_00, 1, true),
        ];
        let ev = evolution(&txs, "2026-11", 3);
        assert_eq!(ev, vec![
            ("2026-09".into(), 100_00),
            ("2026-10".into(), 70_00),
            ("2026-11".into(), 120_00),
        ]);
    }

    #[test]
    fn goal_projection() {
        assert_eq!(months_to_goal(1000_00, 0, 300_00), Some(4));
        assert_eq!(months_to_goal(1000_00, 1000_00, 300_00), Some(0));
        assert_eq!(months_to_goal(1000_00, 0, 0), None);
        assert_eq!(required_monthly(1200_00, 200_00, 5), 200_00);
        assert_eq!(required_monthly(1200_00, 200_00, 0), 1000_00);
    }

    #[test]
    fn forecast_is_linear() {
        assert_eq!(forecast(1000_00, -100_00, 3), vec![900_00, 800_00, 700_00]);
    }

    #[test]
    fn csv_escapes_quotes() {
        let mut t = tx(TxKind::Expense, d(2026, 11, 2), 10_00, 1, true);
        t.description = "Padaria \"Pão\"".into();
        let csv = transactions_csv(&[t], &|_| "Carteira".into(), &|_| "Alimentação".into());
        assert!(csv.contains("\"Padaria \"\"Pão\"\"\""));
        assert!(csv.starts_with("data,tipo"));
    }
}
