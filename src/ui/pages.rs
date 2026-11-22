//! Páginas do GranaFlow.

use chrono::NaiveDate;
use eframe::egui::{self, Color32, RichText};
use granaflow::model::*;
use granaflow::service::*;

use super::charts;
use super::forms::{AccountDraft, BudgetDraft, CategoryDraft, DepositDraft, Form, GoalDraft, TxDraft};
use super::{
    card, kpi, month_bar, month_label, page_header, progress, App, Target, ACCENT, EXPENSE, INCOME, INFO,
};

// ---------------------------------------------------------------- Painel

pub fn dashboard(app: &mut App, ui: &mut egui::Ui) {
    page_header(ui, "Painel", "Sua situação financeira em poucos segundos.");
    if month_bar(ui, &mut app.month) {
        app.budgets = app.db.budget_list(&app.month).unwrap_or_default();
    }
    ui.add_space(8.0);

    let sum = month_summary(&app.txs, &app.month);
    let worth = net_worth(&app.accounts, &app.txs);
    let balance = app.total_balance();

    ui.columns(4, |cols| {
        kpi(&mut cols[0], "Saldo disponível", balance, "contas e carteiras", ACCENT);
        kpi(
            &mut cols[1],
            "Receitas recebidas",
            sum.income_paid,
            &format!("a receber {}", fmt_brl(sum.income_pending)),
            INCOME,
        );
        kpi(
            &mut cols[2],
            "Despesas pagas",
            sum.expense_paid,
            &format!("a pagar {}", fmt_brl(sum.expense_pending)),
            EXPENSE,
        );
        let (color, hint) = if sum.result() >= 0 {
            (ACCENT, "sobrou no mês")
        } else {
            (EXPENSE, "faltou no mês")
        };
        kpi(&mut cols[3], "Resultado do mês", sum.result(), hint, color);
    });

    ui.add_space(10.0);
    ui.columns(2, |cols| {
        card(&mut cols[0], |ui| {
            ui.label(RichText::new("Patrimônio").size(14.0).strong());
            ui.add_space(6.0);
            egui::Grid::new("worth_grid").num_columns(2).spacing([20.0, 4.0]).show(ui, |ui| {
                ui.label("Ativos");
                ui.label(RichText::new(fmt_brl(worth.assets)).color(INCOME));
                ui.end_row();
                ui.label("Dívidas");
                ui.label(RichText::new(fmt_brl(worth.debts)).color(EXPENSE));
                ui.end_row();
                ui.label("Patrimônio líquido");
                ui.label(RichText::new(fmt_brl(worth.net)).strong());
                ui.end_row();
            });
            ui.add_space(4.0);
            ui.weak("Transferências entre contas não alteram o patrimônio.");
        });

        card(&mut cols[1], |ui| {
            ui.label(RichText::new("Metas de economia").size(14.0).strong());
            ui.add_space(6.0);
            if app.goals.is_empty() {
                ui.weak("Nenhuma meta ainda. Crie uma em Metas.");
            }
            for g in app.goals.iter().take(3) {
                let ratio = if g.target_cents > 0 {
                    g.saved_cents as f32 / g.target_cents as f32
                } else {
                    0.0
                };
                ui.horizontal(|ui| {
                    ui.label(&g.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.weak(format!("{:.0}%", ratio * 100.0));
                    });
                });
                progress(ui, ratio, ACCENT, 7.0);
                ui.add_space(4.0);
            }
        });
    });

    ui.add_space(10.0);
    let spend = category_spend(&app.txs, &app.month);
    let bars: Vec<(String, i64, Color32)> = spend
        .iter()
        .take(7)
        .map(|(id, v)| (app.category_name(*id), *v, app.category_color(*id)))
        .collect();
    let evo = evolution(&app.txs, &app.month, 6);
    ui.columns(2, |cols| {
        card(&mut cols[0], |ui| {
            ui.label(RichText::new("Despesas por categoria").size(14.0).strong());
            ui.add_space(6.0);
            charts::bars(ui, &bars);
        });
        card(&mut cols[1], |ui| {
            ui.label(RichText::new("Evolução do saldo · 6 meses").size(14.0).strong());
            ui.add_space(6.0);
            charts::line(ui, &evo, 160.0);
            ui.weak("Fluxo acumulado de receitas menos despesas, sem saldo inicial.");
        });
    });

    ui.add_space(10.0);
    let insight = if sum.income_paid > 0 {
        let pct = sum.expense_paid as f32 / sum.income_paid as f32 * 100.0;
        if pct > 100.0 {
            format!("Você gastou {pct:.0}% do que recebeu neste mês. As despesas passaram das receitas.")
        } else {
            format!("Você comprometeu {pct:.0}% da receita do mês. Sobrou {}.", fmt_brl(sum.result()))
        }
    } else if sum.expense_paid > 0 {
        "Há despesas registradas, mas nenhuma receita recebida neste mês.".to_string()
    } else {
        "Sem movimentações neste mês. Comece registrando uma receita ou despesa.".to_string()
    };
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Situação").size(14.0).strong());
            ui.add_space(8.0);
            let color = if sum.result() >= 0 { ACCENT } else { EXPENSE };
            ui.label(RichText::new(insight).color(color));
        });
    });
}

// ---------------------------------------------------------------- Lançamentos

pub fn transactions(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        page_header(ui, "Lançamentos", "Receitas, despesas e transferências.");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(RichText::new("+ Novo lançamento").strong()).fill(ACCENT))
                .clicked()
            {
                app.new_tx(TxKind::Expense);
            }
        });
    });

    ui.horizontal(|ui| {
        ui.checkbox(&mut app.f_month_only, "Só este mês");
        if month_bar(ui, &mut app.month) {
            app.budgets = app.db.budget_list(&app.month).unwrap_or_default();
        }
    });
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("f_kind")
            .selected_text(match app.f_kind {
                None => "Todos os tipos",
                Some(TxKind::Income) => "Receitas",
                Some(TxKind::Expense) => "Despesas",
                Some(TxKind::Transfer) => "Transferências",
            })
            .width(160.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.f_kind, None, "Todos os tipos");
                ui.selectable_value(&mut app.f_kind, Some(TxKind::Income), "Receitas");
                ui.selectable_value(&mut app.f_kind, Some(TxKind::Expense), "Despesas");
                ui.selectable_value(&mut app.f_kind, Some(TxKind::Transfer), "Transferências");
            });

        egui::ComboBox::from_id_salt("f_acc")
            .selected_text(if app.f_account == 0 {
                "Todas as contas".to_string()
            } else {
                app.account_name(app.f_account)
            })
            .width(170.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.f_account, 0, "Todas as contas");
                for a in &app.accounts {
                    ui.selectable_value(&mut app.f_account, a.id, &a.name);
                }
            });

        ui.add(
            egui::TextEdit::singleline(&mut app.f_text)
                .hint_text("buscar descrição…")
                .desired_width(220.0),
        );
    });
    ui.add_space(8.0);

    let needle = app.f_text.trim().to_lowercase();
    let rows: Vec<Transaction> = app
        .txs
        .iter()
        .filter(|t| {
            (!app.f_month_only || month_key(t.date) == app.month)
                && app.f_kind.map_or(true, |k| t.kind == k)
                && (app.f_account == 0
                    || t.account_id == app.f_account
                    || t.to_account_id == Some(app.f_account))
                && (needle.is_empty() || t.description.to_lowercase().contains(&needle))
        })
        .cloned()
        .collect();

    let mut action: Option<(u8, i64, bool)> = None;
    card(ui, |ui| {
        let total: i64 = rows
            .iter()
            .filter(|t| t.kind != TxKind::Transfer)
            .map(|t| if t.kind == TxKind::Income { t.amount_cents } else { -t.amount_cents })
            .sum();
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} lançamento(s)", rows.len())).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!("resultado filtrado {}", fmt_brl(total)));
            });
        });
        ui.add_space(6.0);
        egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| {
            egui::Grid::new("tx_table")
                .striped(true)
                .num_columns(8)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for h in ["Data", "Tipo", "Descrição", "Categoria", "Conta", "Valor", "Situação", ""] {
                        ui.label(RichText::new(h).strong().size(12.0));
                    }
                    ui.end_row();

                    for t in &rows {
                        ui.label(fmt_date(t.date));
                        let (kind_label, kind_color) = match t.kind {
                            TxKind::Income => ("Receita", INCOME),
                            TxKind::Expense => ("Despesa", EXPENSE),
                            TxKind::Transfer => ("Transferência", INFO),
                        };
                        ui.label(RichText::new(kind_label).color(kind_color).size(12.0));
                        ui.label(if t.description.is_empty() { "—" } else { &t.description });
                        ui.label(t.category_id.map(|c| app.category_name(c)).unwrap_or_else(|| "—".into()));
                        let acc = if let Some(to) = t.to_account_id {
                            format!("{} → {}", app.account_name(t.account_id), app.account_name(to))
                        } else {
                            app.account_name(t.account_id)
                        };
                        ui.label(acc);
                        let color = match t.kind {
                            TxKind::Income => INCOME,
                            TxKind::Expense => EXPENSE,
                            TxKind::Transfer => INFO,
                        };
                        ui.label(
                            RichText::new(format!(
                                "{}{}",
                                if t.kind == TxKind::Income { "+" } else { "" },
                                fmt_brl(t.amount_cents)
                            ))
                            .color(color)
                            .strong(),
                        );
                        if t.kind == TxKind::Transfer {
                            ui.weak("—");
                        } else if ui
                            .small_button(if t.paid { "Pago" } else { "Pendente" })
                            .clicked()
                        {
                            action = Some((3, t.id, t.paid));
                        }
                        ui.horizontal(|ui| {
                            if ui.small_button("Editar").clicked() {
                                action = Some((1, t.id, t.paid));
                            }
                            if ui.small_button("Excluir").clicked() {
                                action = Some((2, t.id, t.paid));
                            }
                        });
                        ui.end_row();
                    }
                });
            if rows.is_empty() {
                ui.add_space(8.0);
                ui.weak("Nenhum lançamento encontrado com esses filtros.");
            }
        });
    });

    if let Some((code, id, paid)) = action {
        match code {
            1 => {
                if let Some(t) = app.txs.iter().find(|t| t.id == id) {
                    app.form = Some(Form::Tx(TxDraft::from_tx(t)));
                }
            }
            2 => {
                let label = app
                    .txs
                    .iter()
                    .find(|t| t.id == id)
                    .map(|t| t.description.clone())
                    .unwrap_or_default();
                app.ask_delete(
                    "Excluir lançamento",
                    format!("Excluir \"{label}\"? Essa ação não pode ser desfeita."),
                    Target::Tx(id),
                );
            }
            3 => {
                if app.db.transaction_set_paid(id, !paid).is_ok() {
                    app.dirty = true;
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------- Orçamento

pub fn budget(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        page_header(ui, "Orçamento", "Limites por categoria e quanto ainda dá para gastar.");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(RichText::new("+ Definir limite").strong()).fill(ACCENT))
                .clicked()
            {
                let category = app.cat_expense.first().map(|c| c.id).unwrap_or(0);
                app.form = Some(Form::Budget(BudgetDraft {
                    month: app.month.clone(),
                    category,
                    limit: String::new(),
                    error: None,
                }));
            }
        });
    });
    if month_bar(ui, &mut app.month) {
        app.budgets = app.db.budget_list(&app.month).unwrap_or_default();
    }
    ui.add_space(8.0);

    let limits: Vec<(i64, i64)> = app.budgets.iter().map(|b| (b.category_id, b.limit_cents)).collect();
    let lines = budget_status(&limits, &app.txs, &app.month);
    let total_limit: i64 = limits.iter().map(|(_, l)| *l).sum();
    let total_spent: i64 = lines.iter().map(|l| l.spent_cents).sum();
    let remaining = total_limit - total_spent;

    ui.columns(3, |cols| {
        kpi(&mut cols[0], "Orçamento do mês", total_limit, "soma dos limites", INFO);
        kpi(&mut cols[1], "Gasto dentro do orçamento", total_spent, "despesas do mês", EXPENSE);
        kpi(
            &mut cols[2],
            "Disponível",
            remaining,
            if remaining >= 0 { "ainda pode gastar" } else { "acima do limite" },
            if remaining >= 0 { ACCENT } else { EXPENSE },
        );
    });
    ui.add_space(10.0);

    if lines.is_empty() {
        card(ui, |ui| {
            ui.label(RichText::new("Nenhum limite definido para este mês.").strong());
            ui.weak("Defina limites por categoria para acompanhar o quanto já gastou.");
        });
        return;
    }

    let mut action: Option<(u8, i64)> = None;
    for line in &lines {
        let name = app.category_name(line.category_id);
        let color = app.category_color(line.category_id);
        let budget_id = app
            .budgets
            .iter()
            .find(|b| b.category_id == line.category_id)
            .map(|b| b.id)
            .unwrap_or(0);
        let ratio = line.ratio();
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(name).size(14.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{:.0}%", ratio * 100.0))
                            .strong()
                            .color(if line.over() { EXPENSE } else { color }),
                    );
                });
            });
            ui.add_space(4.0);
            progress(ui, ratio, if line.over() { EXPENSE } else { color }, 8.0);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.weak(format!(
                    "Gasto {} de {} · restam {}",
                    fmt_brl(line.spent_cents),
                    fmt_brl(line.limit_cents),
                    fmt_brl(line.remaining())
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Remover").clicked() {
                        action = Some((2, budget_id));
                    }
                    if ui.small_button("Editar").clicked() {
                        action = Some((1, line.category_id));
                    }
                });
            });
            if line.over() {
                ui.label(RichText::new("Limite estourado nesta categoria.").color(EXPENSE).size(11.5));
            } else {
                let pct = if line.limit_cents > 0 {
                    (line.spent_cents as f32 / line.limit_cents as f32 * 100.0).round()
                } else {
                    0.0
                };
                let _ = pct;
            }
        });
        ui.add_space(8.0);
    }

    if let Some((code, id)) = action {
        match code {
            1 => {
                let limit = app
                    .budgets
                    .iter()
                    .find(|b| b.category_id == id)
                    .map(|b| b.limit_cents)
                    .unwrap_or(0);
                app.form = Some(Form::Budget(BudgetDraft {
                    month: app.month.clone(),
                    category: id,
                    limit: fmt_brl(limit).replace("R$ ", ""),
                    error: None,
                }));
            }
            2 => app.ask_delete(
                "Remover limite",
                "Remover este limite de orçamento?".to_string(),
                Target::Budget(id),
            ),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------- Metas

pub fn goals(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        page_header(ui, "Metas", "Objetivos de economia e quando serão atingidos.");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(RichText::new("+ Nova meta").strong()).fill(ACCENT))
                .clicked()
            {
                app.form = Some(Form::Goal(GoalDraft::new()));
            }
        });
    });

    let avg = avg_monthly_result(&app.txs, &app.month, 3);
    ui.label(
        RichText::new(format!("Sobra média dos últimos 3 meses: {} por mês", fmt_brl(avg)))
            .size(12.0)
            .weak(),
    );
    ui.add_space(10.0);

    if app.goals.is_empty() {
        card(ui, |ui| {
            ui.label(RichText::new("Nenhuma meta cadastrada.").strong());
            ui.weak("Exemplos: reserva de emergência, viagem, computador novo, entrada do imóvel.");
        });
        return;
    }

    let mut action: Option<(u8, i64, String)> = None;
    let goals = app.goals.clone();
    let mut i = 0usize;
    while i < goals.len() {
        let cols_n = 2.min(goals.len() - i);
        ui.columns(cols_n, |cols| {
            for (k, col) in cols.iter_mut().enumerate() {
                let g = &goals[i + k];
                card(col, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&g.name).size(15.0).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Excluir").clicked() {
                                action = Some((3, g.id, g.name.clone()));
                            }
                            if ui.small_button("Editar").clicked() {
                                action = Some((2, g.id, g.name.clone()));
                            }
                        });
                    });
                    let ratio = if g.target_cents > 0 {
                        (g.saved_cents as f32 / g.target_cents as f32).min(1.0)
                    } else {
                        0.0
                    };
                    ui.add_space(4.0);
                    progress(ui, ratio, ACCENT, 9.0);
                    ui.add_space(4.0);
                    ui.label(format!(
                        "{} de {} · {:.0}%",
                        fmt_brl(g.saved_cents),
                        fmt_brl(g.target_cents),
                        ratio * 100.0
                    ));
                    let remaining = (g.target_cents - g.saved_cents).max(0);
                    match months_to_goal(g.target_cents, g.saved_cents, avg) {
                        Some(0) => {
                            ui.label(RichText::new("Meta atingida!").color(ACCENT).strong());
                        }
                        Some(n) => {
                            ui.weak(format!(
                                "Faltam {} · ~{} meses no ritmo atual",
                                fmt_brl(remaining),
                                n
                            ));
                        }
                        None => {
                            ui.weak(format!(
                                "Faltam {} · sem sobra média para projetar",
                                fmt_brl(remaining)
                            ));
                        }
                    }
                    if let Some(deadline) = g.deadline {
                        let months = months_between(today(), deadline);
                        if months > 0 {
                            ui.weak(format!(
                                "Prazo {} · precisa de {} por mês",
                                fmt_date(deadline),
                                fmt_brl(required_monthly(g.target_cents, g.saved_cents, months))
                            ));
                        } else {
                            ui.weak(format!("Prazo {} · vencido", fmt_date(deadline)));
                        }
                    }
                    ui.add_space(6.0);
                    if ui.button("Guardar").clicked() {
                        action = Some((1, g.id, g.name.clone()));
                    }
                });
            }
        });
        ui.add_space(8.0);
        i += cols_n;
    }

    if let Some((code, id, name)) = action {
        match code {
            1 => {
                app.form = Some(Form::Deposit(DepositDraft {
                    goal_id: id,
                    goal_name: name,
                    amount: String::new(),
                    error: None,
                }));
            }
            2 => {
                if let Some(g) = app.goals.iter().find(|g| g.id == id) {
                    app.form = Some(Form::Goal(GoalDraft::from_goal(g)));
                }
            }
            3 => app.ask_delete(
                "Excluir meta",
                format!("Excluir a meta \"{name}\"?"),
                Target::Goal(id),
            ),
            _ => {}
        }
    }
}

fn months_between(from: NaiveDate, to: NaiveDate) -> i64 {
    let num = |d: NaiveDate, f: &str| d.format(f).to_string().parse::<i64>().unwrap_or(0);
    (num(to, "%Y") - num(from, "%Y")) * 12 + (num(to, "%m") - num(from, "%m"))
}

// ---------------------------------------------------------------- Contas

pub fn accounts(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        page_header(ui, "Contas e patrimônio", "Onde o dinheiro está e quanto você deve.");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(RichText::new("+ Nova conta").strong()).fill(ACCENT))
                .clicked()
            {
                app.form = Some(Form::Account(AccountDraft::new()));
            }
        });
    });

    let worth = net_worth(&app.accounts, &app.txs);
    ui.columns(3, |cols| {
        kpi(&mut cols[0], "Ativos", worth.assets, "saldos positivos", INCOME);
        kpi(&mut cols[1], "Dívidas", worth.debts, "cartões e saldos negativos", EXPENSE);
        kpi(&mut cols[2], "Patrimônio líquido", worth.net, "ativos − dívidas", ACCENT);
    });
    ui.add_space(10.0);

    let rows: Vec<(Account, i64)> = app
        .accounts
        .iter()
        .map(|a| (a.clone(), account_balance(a, &app.txs)))
        .collect();

    let mut action: Option<(u8, i64, String)> = None;
    card(ui, |ui| {
        egui::Grid::new("acc_table").striped(true).num_columns(4).spacing([18.0, 8.0]).show(ui, |ui| {
            for h in ["Conta", "Tipo", "Saldo", ""] {
                ui.label(RichText::new(h).strong().size(12.0));
            }
            ui.end_row();
            for (a, balance) in &rows {
                ui.label(RichText::new(&a.name).strong());
                ui.weak(a.kind.label());
                ui.label(
                    RichText::new(fmt_brl(*balance))
                        .strong()
                        .color(if *balance >= 0 { INCOME } else { EXPENSE }),
                );
                ui.horizontal(|ui| {
                    if ui.small_button("Editar").clicked() {
                        action = Some((1, a.id, a.name.clone()));
                    }
                    if ui.small_button("Arquivar").clicked() {
                        action = Some((2, a.id, a.name.clone()));
                    }
                });
                ui.end_row();
            }
        });
        if rows.is_empty() {
            ui.weak("Nenhuma conta ativa.");
        }
        ui.add_space(4.0);
        ui.weak("Cartões de crédito aparecem como dívida quando o saldo fica negativo.");
    });

    if let Some((code, id, name)) = action {
        match code {
            1 => {
                if let Some(a) = app.accounts.iter().find(|a| a.id == id) {
                    app.form = Some(Form::Account(AccountDraft::from_account(a)));
                }
            }
            2 => app.ask_delete(
                "Arquivar conta",
                format!("Arquivar a conta \"{name}\"? Os lançamentos continuam no histórico."),
                Target::Account(id),
            ),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------- Relatórios

pub fn reports(app: &mut App, ui: &mut egui::Ui) {
    page_header(ui, "Relatórios", "Resumo mensal, categorias e exportação dos seus dados.");
    if month_bar(ui, &mut app.month) {
        app.budgets = app.db.budget_list(&app.month).unwrap_or_default();
    }
    ui.add_space(8.0);

    let sum = month_summary(&app.txs, &app.month);
    let prev_key = shift_month(&app.month, -1);
    let prev = month_summary(&app.txs, &prev_key);
    let delta = |cur: i64, old: i64| -> String {
        if old == 0 {
            "—".into()
        } else {
            let pct = (cur - old) as f32 / old as f32 * 100.0;
            format!("{pct:+.0}% vs {}", month_label(&prev_key))
        }
    };

    card(ui, |ui| {
        ui.label(RichText::new(format!("Relatório de {}", month_label(&app.month))).size(15.0).strong());
        ui.add_space(8.0);
        egui::Grid::new("rep_sum").num_columns(3).spacing([24.0, 6.0]).show(ui, |ui| {
            for h in ["Indicador", "Valor", "Comparação"] {
                ui.label(RichText::new(h).strong().size(12.0));
            }
            ui.end_row();
            ui.label("Receitas recebidas");
            ui.label(RichText::new(fmt_brl(sum.income_paid)).color(INCOME));
            ui.weak(delta(sum.income_paid, prev.income_paid));
            ui.end_row();
            ui.label("Despesas pagas");
            ui.label(RichText::new(fmt_brl(sum.expense_paid)).color(EXPENSE));
            ui.weak(delta(sum.expense_paid, prev.expense_paid));
            ui.end_row();
            ui.label("Resultado");
            ui.label(
                RichText::new(fmt_brl(sum.result()))
                    .strong()
                    .color(if sum.result() >= 0 { ACCENT } else { EXPENSE }),
            );
            ui.weak(delta(sum.result(), prev.result()));
            ui.end_row();
            ui.label("A pagar");
            ui.label(fmt_brl(sum.expense_pending));
            ui.weak("");
            ui.end_row();
            ui.label("A receber");
            ui.label(fmt_brl(sum.income_pending));
            ui.weak("");
            ui.end_row();
        });
    });
    ui.add_space(10.0);

    let spend = category_spend(&app.txs, &app.month);
    let prev_spend = category_spend(&app.txs, &prev_key);
    let total_spend: i64 = spend.iter().map(|(_, v)| *v).sum();
    card(ui, |ui| {
        ui.label(RichText::new("Gastos por categoria").size(15.0).strong());
        ui.add_space(6.0);
        if spend.is_empty() {
            ui.weak("Sem despesas neste mês.");
        }
        egui::Grid::new("rep_cat").striped(true).num_columns(4).spacing([18.0, 6.0]).show(ui, |ui| {
            for h in ["Categoria", "Gasto", "% do total", "Mês anterior"] {
                ui.label(RichText::new(h).strong().size(12.0));
            }
            ui.end_row();
            for (id, value) in &spend {
                let pct = if total_spend > 0 {
                    *value as f32 / total_spend as f32 * 100.0
                } else {
                    0.0
                };
                ui.label(RichText::new(app.category_name(*id)).color(app.category_color(*id)));
                ui.label(fmt_brl(*value));
                ui.label(format!("{pct:.0}%"));
                let old = prev_spend.iter().find(|(c, _)| c == id).map(|(_, v)| *v).unwrap_or(0);
                ui.weak(delta(*value, old));
                ui.end_row();
            }
        });
    });
    ui.add_space(10.0);

    card(ui, |ui| {
        ui.label(RichText::new("Exportar e proteger").size(15.0).strong());
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Exportar CSV").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(format!("granaflow-{}.csv", app.month))
                    .add_filter("CSV", &["csv"])
                    .save_file()
                {
                    let csv = transactions_csv(&app.txs, &|id| app.account_name(id), &|id| app.category_name(id));
                    match std::fs::write(&path, csv) {
                        Ok(()) => app.toast(format!("CSV salvo em {}", path.display())),
                        Err(e) => app.toast(format!("Erro ao salvar CSV: {e}")),
                    }
                }
            }
            if ui.button("Exportar JSON").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("granaflow-dados.json")
                    .add_filter("JSON", &["json"])
                    .save_file()
                {
                    let dump = serde_json::json!({
                        "app": "GranaFlow",
                        "versao": 1,
                        "gerado_em": today().to_string(),
                        "accounts": &app.accounts,
                        "categories": [&app.cat_income, &app.cat_expense],
                        "transactions": &app.txs,
                        "budgets": &app.budgets,
                        "goals": &app.goals,
                    });
                    match serde_json::to_string_pretty(&dump)
                        .map_err(|e| e.to_string())
                        .and_then(|s| std::fs::write(&path, s).map_err(|e| e.to_string()))
                    {
                        Ok(()) => app.toast(format!("JSON salvo em {}", path.display())),
                        Err(e) => app.toast(format!("Erro ao salvar JSON: {e}")),
                    }
                }
            }
            if ui.button("Backup do banco").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(format!("granaflow-backup-{}.db", today()))
                    .add_filter("SQLite", &["db"])
                    .save_file()
                {
                    match app.db.backup_to(&path) {
                        Ok(()) => app.toast(format!("Backup salvo em {}", path.display())),
                        Err(e) => app.toast(format!("Erro no backup: {e}")),
                    }
                }
            }
        });
        ui.add_space(4.0);
        ui.weak("Os arquivos ficam só na sua máquina. Nada é enviado para a internet.");
    });
}

// ---------------------------------------------------------------- Previsões

pub fn forecast(app: &mut App, ui: &mut egui::Ui) {
    page_header(ui, "Previsões", "Simulações com premissas explícitas — não são garantias.");
    ui.add_space(4.0);

    let balance = app.total_balance();
    let avg = avg_monthly_result(&app.txs, &app.month, 3);

    card(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("Horizonte (meses)");
            ui.add(egui::DragValue::new(&mut app.fc_months).range(1..=36).speed(0.2));
            ui.add_space(12.0);
            ui.label("Resultado mensal (R$)");
            ui.add(
                egui::TextEdit::singleline(&mut app.fc_result)
                    .hint_text(format!("média: {}", fmt_brl(avg)))
                    .desired_width(150.0),
            );
            ui.add_space(12.0);
            ui.label("Despesa extra única (R$)");
            ui.add(
                egui::TextEdit::singleline(&mut app.fc_extra)
                    .hint_text("0,00")
                    .desired_width(120.0),
            );
        });
        ui.add_space(4.0);
        ui.weak("Em branco no resultado mensal = média dos últimos 3 meses.");
    });
    ui.add_space(10.0);

    let monthly = parse_brl(&app.fc_result).unwrap_or(avg) - parse_brl(&app.fc_extra).unwrap_or(0);
    let points: Vec<(String, i64)> = granaflow::service::forecast(balance, monthly, app.fc_months as u32)
        .into_iter()
        .enumerate()
        .map(|(i, v)| (format!("M+{}", i + 1), v))
        .collect();

    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Saldo projetado").size(15.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!("partindo de {}", fmt_brl(balance)));
            });
        });
        ui.add_space(6.0);
        charts::line(ui, &points, 190.0);
    });
    ui.add_space(10.0);

    let mut rows: Vec<(String, i64)> = vec![("Hoje".into(), balance)];
    rows.extend(points.iter().cloned());
    card(ui, |ui| {
        ui.label(RichText::new("Mês a mês").size(15.0).strong());
        ui.add_space(6.0);
        egui::Grid::new("fc_table").striped(true).num_columns(3).spacing([24.0, 5.0]).show(ui, |ui| {
            for h in ["Período", "Saldo projetado", "Variação"] {
                ui.label(RichText::new(h).strong().size(12.0));
            }
            ui.end_row();
            let mut last = balance;
            for (label, v) in &rows {
                ui.label(label);
                ui.label(
                    RichText::new(fmt_brl(*v))
                        .color(if *v >= 0 { INCOME } else { EXPENSE })
                        .strong(),
                );
                let diff = *v - last;
                ui.weak(if diff == 0 { "—".to_string() } else { fmt_brl(diff) });
                ui.end_row();
                last = *v;
            }
        });
    });
    ui.add_space(10.0);

    card(ui, |ui| {
        ui.label(RichText::new("Premissas").size(14.0).strong());
        ui.weak("• O resultado mensal informado se repete sem variação em todos os meses.");
        ui.weak("• Não há inflação, rendimento de investimentos nem imprevistos no cálculo.");
        ui.weak("• A despesa extra é descontada uma única vez no primeiro mês.");
        ui.weak("• A projeção serve para planejar, não para prever o futuro com certeza.");
    });
    ui.add_space(10.0);

    if !app.goals.is_empty() {
        card(ui, |ui| {
            ui.label(RichText::new("Quando cada meta seria atingida").size(14.0).strong());
            ui.add_space(6.0);
            for g in &app.goals {
                let monthly_saving = monthly.max(0);
                let text = match months_to_goal(g.target_cents, g.saved_cents, monthly_saving) {
                    Some(0) => "atingida".to_string(),
                    Some(n) => format!("~{n} meses"),
                    None => "sem aporte projetado".to_string(),
                };
                ui.horizontal(|ui| {
                    ui.label(&g.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(text);
                    });
                });
            }
        });
    }
}

// ---------------------------------------------------------------- Configurações

pub fn settings(app: &mut App, ui: &mut egui::Ui) {
    page_header(ui, "Configurações", "Aparência, dados e categorias.");
    ui.add_space(4.0);

    card(ui, |ui| {
        ui.label(RichText::new("Aparência").size(15.0).strong());
        ui.add_space(6.0);
        let mut dark = app.dark;
        if ui.checkbox(&mut dark, "Tema escuro").changed() {
            app.dark = dark;
            super::apply_theme(ui.ctx(), dark);
            let _ = app.db.setting_set("theme", if dark { "dark" } else { "light" });
        }
        ui.weak("A moeda do aplicativo é o Real (R$). Valores são guardados em centavos.");
    });
    ui.add_space(10.0);

    card(ui, |ui| {
        ui.label(RichText::new("Seus dados").size(15.0).strong());
        ui.add_space(4.0);
        ui.label(format!("Banco de dados: {}", app.db_path.display()));
        ui.weak("Tudo fica neste arquivo. Sem nuvem, sem conta, sem internet.");
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Backup agora").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(format!("granaflow-backup-{}.db", today()))
                    .add_filter("SQLite", &["db"])
                    .save_file()
                {
                    match app.db.backup_to(&path) {
                        Ok(()) => app.toast(format!("Backup salvo em {}", path.display())),
                        Err(e) => app.toast(format!("Erro no backup: {e}")),
                    }
                }
            }
            if ui.button("Restaurar backup").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("SQLite", &["db"])
                    .pick_file()
                {
                    let auto = app
                        .db_path
                        .with_file_name(format!("granaflow-pre-restore-{}.db", today()));
                    let backup_ok = app.db.backup_to(&auto).is_ok();
                    match app.db.restore_from(&path) {
                        Ok(()) => {
                            app.dirty = true;
                            app.toast(if backup_ok {
                                format!("Backup restaurado. Cópia anterior em {}", auto.display())
                            } else {
                                "Backup restaurado.".to_string()
                            });
                        }
                        Err(e) => app.toast(format!("Erro ao restaurar: {e}")),
                    }
                }
            }
        });
    });
    ui.add_space(10.0);

    let mut action: Option<(u8, i64, String)> = None;
    ui.columns(2, |cols| {
        for (idx, (kind, cats)) in [
            (TxKind::Income, &app.cat_income),
            (TxKind::Expense, &app.cat_expense),
        ]
        .into_iter()
        .enumerate()
        {
            card(&mut cols[idx], |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if kind == TxKind::Income {
                            "Categorias de receita"
                        } else {
                            "Categorias de despesa"
                        })
                        .size(15.0)
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("+ Adicionar").clicked() {
                            action = Some((1, 0, String::new()));
                        }
                    });
                });
                ui.add_space(6.0);
                for c in cats {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.0, super::forms::hex_color(&c.color));
                        ui.label(&c.name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Excluir").clicked() {
                                action = Some((2, c.id, c.name.clone()));
                            }
                        });
                    });
                }
            });
        }
    });
    if let Some((code, id, name)) = action {
        match code {
            1 => {
                let kind = if app.cat_income.len() <= app.cat_expense.len() {
                    TxKind::Income
                } else {
                    TxKind::Expense
                };
                app.form = Some(Form::Category(CategoryDraft::new(kind)));
            }
            2 => app.ask_delete(
                "Excluir categoria",
                format!("Excluir \"{name}\"? Os lançamentos ficam sem categoria."),
                Target::Category(id),
            ),
            _ => {}
        }
    }
    ui.add_space(10.0);

    card(ui, |ui| {
        ui.label(RichText::new("Sobre o GranaFlow").size(15.0).strong());
        ui.add_space(4.0);
        ui.label("Versão 0.1.0 · Rust + egui + SQLite");
        ui.weak("Local-first: seus dados nunca saem deste computador sem você mandar.");
        ui.weak("Não armazene senhas bancárias aqui. Este app não acessa bancos.");
    });
}

