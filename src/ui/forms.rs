//! Janelas de formulário (novo/editar) e confirmação de exclusão.

use eframe::egui::{self, RichText};
use granaflow::db::Database;
use granaflow::model::*;

use super::{App, EXPENSE, INCOME};

pub enum Form {
    Tx(TxDraft),
    Account(AccountDraft),
    Category(CategoryDraft),
    Budget(BudgetDraft),
    Goal(GoalDraft),
    Deposit(DepositDraft),
}

impl Form {
    fn title(&self) -> &'static str {
        match self {
            Form::Tx(d) if d.id == 0 => "Novo lançamento",
            Form::Tx(_) => "Editar lançamento",
            Form::Account(d) if d.id == 0 => "Nova conta",
            Form::Account(_) => "Editar conta",
            Form::Category(_) => "Nova categoria",
            Form::Budget(_) => "Limite de orçamento",
            Form::Goal(d) if d.id == 0 => "Nova meta",
            Form::Goal(_) => "Editar meta",
            Form::Deposit(_) => "Guardar na meta",
        }
    }

    fn error(&self) -> Option<&String> {
        match self {
            Form::Tx(d) => d.error.as_ref(),
            Form::Account(d) => d.error.as_ref(),
            Form::Category(d) => d.error.as_ref(),
            Form::Budget(d) => d.error.as_ref(),
            Form::Goal(d) => d.error.as_ref(),
            Form::Deposit(d) => d.error.as_ref(),
        }
    }

    fn set_error(&mut self, msg: String) {
        match self {
            Form::Tx(d) => d.error = Some(msg),
            Form::Account(d) => d.error = Some(msg),
            Form::Category(d) => d.error = Some(msg),
            Form::Budget(d) => d.error = Some(msg),
            Form::Goal(d) => d.error = Some(msg),
            Form::Deposit(d) => d.error = Some(msg),
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Form::Tx(d) => d.validate(),
            Form::Account(d) => d.validate(),
            Form::Category(d) => d.validate(),
            Form::Budget(d) => d.validate(),
            Form::Goal(d) => d.validate(),
            Form::Deposit(d) => d.validate(),
        }
    }

    fn apply(&self, db: &Database) -> rusqlite::Result<()> {
        match self {
            Form::Tx(d) => d.apply(db),
            Form::Account(d) => d.apply(db),
            Form::Category(d) => d.apply(db),
            Form::Budget(d) => d.apply(db),
            Form::Goal(d) => d.apply(db),
            Form::Deposit(d) => d.apply(db),
        }
    }
}

// ---------- Lançamento ----------

pub struct TxDraft {
    pub id: i64,
    pub kind: TxKind,
    pub date: String,
    pub amount: String,
    pub account: i64,
    pub to_account: i64,
    pub category: i64,
    pub description: String,
    pub paid: bool,
    pub error: Option<String>,
}

impl TxDraft {
    pub fn new(kind: TxKind, account: i64, category: i64, month: &str) -> Self {
        let today = today();
        let date = if month_key(today) == month {
            today
        } else {
            month_start(month).unwrap_or(today)
        };
        TxDraft {
            id: 0,
            kind,
            date: fmt_date(date),
            amount: String::new(),
            account,
            to_account: 0,
            category,
            description: String::new(),
            paid: true,
            error: None,
        }
    }

    pub fn from_tx(t: &Transaction) -> Self {
        TxDraft {
            id: t.id,
            kind: t.kind,
            date: fmt_date(t.date),
            amount: fmt_brl(t.amount_cents).replace("R$ ", ""),
            account: t.account_id,
            to_account: t.to_account_id.unwrap_or(0),
            category: t.category_id.unwrap_or(0),
            description: t.description.clone(),
            paid: t.paid,
            error: None,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.description.trim().is_empty() {
            return Err("Informe uma descrição.".into());
        }
        let Some(v) = parse_brl(&self.amount) else {
            return Err("Valor inválido. Use 1.234,56.".into());
        };
        if v == 0 {
            return Err("O valor precisa ser maior que zero.".into());
        }
        if self.date.trim().is_empty() || parse_date(&self.date).is_none() {
            return Err("Data inválida. Use dd/mm/aaaa.".into());
        }
        if self.account == 0 {
            return Err("Escolha a conta.".into());
        }
        if self.kind == TxKind::Transfer && (self.to_account == 0 || self.to_account == self.account) {
            return Err("Transferência precisa de duas contas diferentes.".into());
        }
        Ok(())
    }

    fn apply(&self, db: &Database) -> rusqlite::Result<()> {
        let tx = Transaction {
            id: self.id,
            kind: self.kind,
            date: parse_date(&self.date).unwrap_or_else(today),
            amount_cents: parse_brl(&self.amount).unwrap_or(0),
            account_id: self.account,
            to_account_id: if self.kind == TxKind::Transfer && self.to_account > 0 {
                Some(self.to_account)
            } else {
                None
            },
            category_id: if self.category > 0 { Some(self.category) } else { None },
            description: self.description.trim().to_string(),
            paid: self.paid,
        };
        if self.id == 0 {
            db.transaction_insert(&tx)?;
        } else {
            db.transaction_update(&tx)?;
        }
        Ok(())
    }
}

// ---------- Conta ----------

pub struct AccountDraft {
    pub id: i64,
    pub name: String,
    pub kind: AccountKind,
    pub initial: String,
    pub error: Option<String>,
}

impl AccountDraft {
    pub fn new() -> Self {
        AccountDraft {
            id: 0,
            name: String::new(),
            kind: AccountKind::Checking,
            initial: String::new(),
            error: None,
        }
    }

    pub fn from_account(a: &Account) -> Self {
        AccountDraft {
            id: a.id,
            name: a.name.clone(),
            kind: a.kind,
            initial: fmt_brl(a.initial_cents).replace("R$ ", ""),
            error: None,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Informe o nome da conta.".into());
        }
        if !self.initial.trim().is_empty() && parse_brl(&self.initial).is_none() {
            return Err("Saldo inicial inválido.".into());
        }
        Ok(())
    }

    fn apply(&self, db: &Database) -> rusqlite::Result<()> {
        let initial = parse_brl(&self.initial).unwrap_or(0);
        if self.id == 0 {
            db.account_insert(self.name.trim(), self.kind, initial)?;
        } else {
            db.account_update(self.id, self.name.trim(), self.kind, initial)?;
        }
        Ok(())
    }
}

// ---------- Categoria ----------

pub struct CategoryDraft {
    pub name: String,
    pub kind: TxKind,
    pub color: String,
    pub error: Option<String>,
}

impl CategoryDraft {
    pub fn new(kind: TxKind) -> Self {
        CategoryDraft {
            name: String::new(),
            kind,
            color: if kind == TxKind::Income { "#10B981".into() } else { "#EF4444".into() },
            error: None,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Informe o nome da categoria.".into());
        }
        if !self.color.starts_with('#') || self.color.len() != 7 {
            return Err("Cor no formato #RRGGBB.".into());
        }
        Ok(())
    }

    fn apply(&self, db: &Database) -> rusqlite::Result<()> {
        db.category_insert(self.name.trim(), self.kind, &self.color)?;
        Ok(())
    }
}

// ---------- Orçamento ----------

pub struct BudgetDraft {
    pub month: String,
    pub category: i64,
    pub limit: String,
    pub error: Option<String>,
}

impl BudgetDraft {
    fn validate(&self) -> Result<(), String> {
        if self.category == 0 {
            return Err("Escolha a categoria.".into());
        }
        match parse_brl(&self.limit) {
            Some(v) if v > 0 => Ok(()),
            _ => Err("Limite inválido. Use 1.234,56.".into()),
        }
    }

    fn apply(&self, db: &Database) -> rusqlite::Result<()> {
        db.budget_set(&self.month, self.category, parse_brl(&self.limit).unwrap_or(0))
    }
}

// ---------- Meta ----------

pub struct GoalDraft {
    pub id: i64,
    pub name: String,
    pub target: String,
    pub saved: String,
    pub deadline: String,
    pub error: Option<String>,
}

impl GoalDraft {
    pub fn new() -> Self {
        GoalDraft {
            id: 0,
            name: String::new(),
            target: String::new(),
            saved: String::new(),
            deadline: String::new(),
            error: None,
        }
    }

    pub fn from_goal(g: &Goal) -> Self {
        GoalDraft {
            id: g.id,
            name: g.name.clone(),
            target: fmt_brl(g.target_cents).replace("R$ ", ""),
            saved: fmt_brl(g.saved_cents).replace("R$ ", ""),
            deadline: g.deadline.map(fmt_date).unwrap_or_default(),
            error: None,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Dê um nome para a meta.".into());
        }
        match parse_brl(&self.target) {
            Some(v) if v > 0 => {}
            _ => return Err("Valor da meta inválido.".into()),
        }
        if !self.saved.trim().is_empty() && parse_brl(&self.saved).is_none() {
            return Err("Valor guardado inválido.".into());
        }
        if !self.deadline.trim().is_empty() && parse_date(&self.deadline).is_none() {
            return Err("Prazo inválido. Use dd/mm/aaaa.".into());
        }
        Ok(())
    }

    fn apply(&self, db: &Database) -> rusqlite::Result<()> {
        let goal = Goal {
            id: self.id,
            name: self.name.trim().to_string(),
            target_cents: parse_brl(&self.target).unwrap_or(0),
            saved_cents: parse_brl(&self.saved).unwrap_or(0),
            deadline: parse_date(&self.deadline),
        };
        if self.id == 0 {
            db.goal_insert(&goal)?;
        } else {
            db.goal_update(&goal)?;
        }
        Ok(())
    }
}

// ---------- Aporte em meta ----------

pub struct DepositDraft {
    pub goal_id: i64,
    pub goal_name: String,
    pub amount: String,
    pub error: Option<String>,
}

impl DepositDraft {
    fn validate(&self) -> Result<(), String> {
        match parse_brl(&self.amount) {
            Some(v) if v > 0 => Ok(()),
            _ => Err("Valor inválido.".into()),
        }
    }

    fn apply(&self, db: &Database) -> rusqlite::Result<()> {
        db.goal_deposit(self.goal_id, parse_brl(&self.amount).unwrap_or(0))
    }
}

// ---------- Render ----------

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(mut form) = app.form.take() else {
        return;
    };
    let accounts = app.accounts.clone();
    let cats_income = app.cat_income.clone();
    let cats_expense = app.cat_expense.clone();
    let goals = app.goals.clone();
    let month = app.month.clone();
    let mut action = 0u8;

    egui::Window::new(form.title())
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(430.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);
            match &mut form {
                Form::Tx(d) => tx_ui(ui, d, &accounts, &cats_income, &cats_expense),
                Form::Account(d) => account_ui(ui, d),
                Form::Category(d) => category_ui(ui, d),
                Form::Budget(d) => {
                    d.month = month.clone();
                    budget_ui(ui, d, &cats_expense);
                }
                Form::Goal(d) => goal_ui(ui, d),
                Form::Deposit(d) => {
                    let saved = goals.iter().find(|g| g.id == d.goal_id);
                    ui.label(format!("Meta: {}", d.goal_name));
                    if let Some(g) = saved {
                        ui.weak(format!(
                            "Guardado {} de {}",
                            fmt_brl(g.saved_cents),
                            fmt_brl(g.target_cents)
                        ));
                    }
                    ui.add_space(6.0);
                    ui.label("Valor do aporte (R$)");
                    ui.add(
                        egui::TextEdit::singleline(&mut d.amount)
                            .hint_text("0,00")
                            .desired_width(160.0),
                    );
                }
            }
            if let Some(err) = form.error() {
                ui.add_space(6.0);
                ui.colored_label(EXPENSE, err);
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(RichText::new("Salvar").strong()).fill(super::ACCENT))
                    .clicked()
                {
                    action = 1;
                }
                if ui.button("Cancelar").clicked() {
                    action = 2;
                }
            });
        });

    if action == 1 {
        match form.validate() {
            Ok(()) => match form.apply(&app.db) {
                Ok(()) => {
                    app.dirty = true;
                    app.budgets = app.db.budget_list(&app.month).unwrap_or_default();
                    app.toast("Salvo com sucesso");
                }
                Err(e) => {
                    form.set_error(format!("Erro ao salvar: {e}"));
                    app.form = Some(form);
                }
            },
            Err(e) => {
                form.set_error(e);
                app.form = Some(form);
            }
        }
    } else if action == 2 {
        app.form = None;
    } else {
        app.form = Some(form);
    }
}

fn tx_ui(
    ui: &mut egui::Ui,
    d: &mut TxDraft,
    accounts: &[Account],
    cats_income: &[Category],
    cats_expense: &[Category],
) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut d.kind, TxKind::Income, "Receita");
        ui.selectable_value(&mut d.kind, TxKind::Expense, "Despesa");
        ui.selectable_value(&mut d.kind, TxKind::Transfer, "Transferência");
    });
    ui.add_space(6.0);
    egui::Grid::new("tx_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
        ui.label("Descrição");
        ui.add(
            egui::TextEdit::singleline(&mut d.description)
                .hint_text("Ex.: Mercado do mês")
                .desired_width(260.0),
        );
        ui.end_row();

        ui.label("Valor (R$)");
        ui.add(
            egui::TextEdit::singleline(&mut d.amount)
                .hint_text("0,00")
                .desired_width(160.0),
        );
        ui.end_row();

        ui.label("Data");
        ui.add(
            egui::TextEdit::singleline(&mut d.date)
                .hint_text("dd/mm/aaaa")
                .desired_width(120.0),
        );
        ui.end_row();

        ui.label(if d.kind == TxKind::Transfer { "Da conta" } else { "Conta" });
        account_combo(ui, "tx_acc", &mut d.account, accounts);
        ui.end_row();

        if d.kind == TxKind::Transfer {
            ui.label("Para a conta");
            account_combo(ui, "tx_acc2", &mut d.to_account, accounts);
            ui.end_row();
        } else {
            ui.label("Categoria");
            let cats = if d.kind == TxKind::Income { cats_income } else { cats_expense };
            category_combo(ui, "tx_cat", &mut d.category, cats);
            ui.end_row();
        }

        ui.label("Situação");
        ui.horizontal(|ui| {
            ui.radio_value(&mut d.paid, true, if d.kind == TxKind::Income { "Recebido" } else { "Pago" });
            ui.radio_value(&mut d.paid, false, "Pendente");
        });
        ui.end_row();
    });
    if d.kind == TxKind::Transfer {
        ui.add_space(4.0);
        ui.colored_label(INCOME, "Transferências não entram como receita nem despesa.");
    }
}

fn account_ui(ui: &mut egui::Ui, d: &mut AccountDraft) {
    egui::Grid::new("acc_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
        ui.label("Nome");
        ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(240.0));
        ui.end_row();
        ui.label("Tipo");
        egui::ComboBox::from_id_salt("acc_kind")
            .selected_text(d.kind.label())
            .width(220.0)
            .show_ui(ui, |ui| {
                for k in AccountKind::ALL {
                    ui.selectable_value(&mut d.kind, k, k.label());
                }
            });
        ui.end_row();
        ui.label("Saldo inicial (R$)");
        ui.add(
            egui::TextEdit::singleline(&mut d.initial)
                .hint_text("0,00")
                .desired_width(160.0),
        );
        ui.end_row();
    });
}

fn category_ui(ui: &mut egui::Ui, d: &mut CategoryDraft) {
    egui::Grid::new("cat_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
        ui.label("Nome");
        ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(240.0));
        ui.end_row();
        ui.label("Cor");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut d.color).desired_width(90.0));
            let c = hex_color(&d.color);
            let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
            ui.painter().rect_filled(r, egui::Rounding::same(4.0), c);
        });
        ui.end_row();
    });
}

fn budget_ui(ui: &mut egui::Ui, d: &mut BudgetDraft, cats: &[Category]) {
    ui.label(format!("Mês: {}", d.month));
    ui.add_space(4.0);
    ui.label("Categoria de despesa");
    category_combo(ui, "bud_cat", &mut d.category, cats);
    ui.add_space(6.0);
    ui.label("Limite mensal (R$)");
    ui.add(
        egui::TextEdit::singleline(&mut d.limit)
            .hint_text("0,00")
            .desired_width(160.0),
    );
}

fn goal_ui(ui: &mut egui::Ui, d: &mut GoalDraft) {
    egui::Grid::new("goal_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
        ui.label("Nome");
        ui.add(
            egui::TextEdit::singleline(&mut d.name)
                .hint_text("Ex.: Reserva de emergência")
                .desired_width(260.0),
        );
        ui.end_row();
        ui.label("Valor da meta (R$)");
        ui.add(
            egui::TextEdit::singleline(&mut d.target)
                .hint_text("0,00")
                .desired_width(160.0),
        );
        ui.end_row();
        ui.label("Já guardado (R$)");
        ui.add(
            egui::TextEdit::singleline(&mut d.saved)
                .hint_text("0,00")
                .desired_width(160.0),
        );
        ui.end_row();
        ui.label("Prazo (opcional)");
        ui.add(
            egui::TextEdit::singleline(&mut d.deadline)
                .hint_text("dd/mm/aaaa")
                .desired_width(120.0),
        );
        ui.end_row();
    });
}

fn account_combo(ui: &mut egui::Ui, salt: &str, id: &mut i64, accounts: &[Account]) {
    let current = accounts
        .iter()
        .find(|a| a.id == *id)
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "Escolha a conta".into());
    egui::ComboBox::from_id_salt(salt)
        .selected_text(current)
        .width(220.0)
        .show_ui(ui, |ui| {
            for a in accounts {
                ui.selectable_value(id, a.id, &a.name);
            }
        });
}

fn category_combo(ui: &mut egui::Ui, salt: &str, id: &mut i64, cats: &[Category]) {
    let current = cats
        .iter()
        .find(|c| c.id == *id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Sem categoria".into());
    egui::ComboBox::from_id_salt(salt)
        .selected_text(current)
        .width(220.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(id, 0, "Sem categoria");
            for c in cats {
                ui.selectable_value(id, c.id, &c.name);
            }
        });
}

pub fn hex_color(s: &str) -> egui::Color32 {
    let t = s.trim_start_matches('#');
    if t.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&t[0..2], 16),
            u8::from_str_radix(&t[2..4], 16),
            u8::from_str_radix(&t[4..6], 16),
        ) {
            return egui::Color32::from_rgb(r, g, b);
        }
    }
    egui::Color32::from_rgb(100, 116, 139)
}
