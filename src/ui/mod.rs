//! Shell do aplicativo: estado, navegação, tema e janelas auxiliares.

pub mod charts;
pub mod forms;
pub mod pages;

use std::path::PathBuf;
use std::time::Instant;

use eframe::egui::{self, Color32, RichText};
use granaflow::db::Database;
use granaflow::model::*;
use granaflow::service::{account_balance, net_worth};

pub const ACCENT: Color32 = Color32::from_rgb(16, 185, 129);
pub const INCOME: Color32 = Color32::from_rgb(52, 211, 153);
pub const EXPENSE: Color32 = Color32::from_rgb(248, 113, 113);
pub const INFO: Color32 = Color32::from_rgb(96, 165, 250);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Transactions,
    Budget,
    Goals,
    Accounts,
    Reports,
    Forecast,
    Settings,
}

impl Page {
    pub const ALL: [Page; 8] = [
        Page::Dashboard,
        Page::Transactions,
        Page::Budget,
        Page::Goals,
        Page::Accounts,
        Page::Reports,
        Page::Forecast,
        Page::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Dashboard => "Painel",
            Page::Transactions => "Lançamentos",
            Page::Budget => "Orçamento",
            Page::Goals => "Metas",
            Page::Accounts => "Contas e patrimônio",
            Page::Reports => "Relatórios",
            Page::Forecast => "Previsões",
            Page::Settings => "Configurações",
        }
    }
}

pub struct Toast {
    pub msg: String,
    pub at: Instant,
}

pub enum Target {
    Tx(i64),
    Account(i64),
    Goal(i64),
    Category(i64),
    Budget(i64),
}

pub struct Confirm {
    pub title: String,
    pub message: String,
    pub target: Target,
}

pub struct App {
    pub db: Database,
    pub db_path: PathBuf,
    pub page: Page,
    pub month: String,
    pub dark: bool,
    pub accounts: Vec<Account>,
    pub cat_income: Vec<Category>,
    pub cat_expense: Vec<Category>,
    pub txs: Vec<Transaction>,
    pub goals: Vec<Goal>,
    pub budgets: Vec<Budget>,
    pub form: Option<forms::Form>,
    pub confirm: Option<Confirm>,
    pub toast: Option<Toast>,
    pub dirty: bool,
    // filtros da lista de lançamentos
    pub f_kind: Option<TxKind>,
    pub f_account: i64,
    pub f_text: String,
    pub f_month_only: bool,
    // previsões
    pub fc_months: i32,
    pub fc_result: String,
    pub fc_extra: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, db: Database, db_path: PathBuf) -> Self {
        let dark = db
            .setting_get("theme")
            .map(|v| v != "light")
            .unwrap_or(true);
        apply_theme(&cc.egui_ctx, dark);
        let mut app = App {
            db,
            db_path,
            page: Page::Dashboard,
            month: month_key(today()),
            dark,
            accounts: Vec::new(),
            cat_income: Vec::new(),
            cat_expense: Vec::new(),
            txs: Vec::new(),
            goals: Vec::new(),
            budgets: Vec::new(),
            form: None,
            confirm: None,
            toast: None,
            dirty: true,
            f_kind: None,
            f_account: 0,
            f_text: String::new(),
            f_month_only: true,
            fc_months: 6,
            fc_result: String::new(),
            fc_extra: String::new(),
        };
        app.reload();
        app
    }

    pub fn reload(&mut self) {
        self.accounts = self.db.account_list().unwrap_or_default();
        self.cat_income = self.db.category_list(TxKind::Income).unwrap_or_default();
        self.cat_expense = self.db.category_list(TxKind::Expense).unwrap_or_default();
        self.txs = self.db.transaction_list().unwrap_or_default();
        self.goals = self.db.goal_list().unwrap_or_default();
        self.budgets = self.db.budget_list(&self.month).unwrap_or_default();
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some(Toast {
            msg: msg.into(),
            at: Instant::now(),
        });
    }

    pub fn category_name(&self, id: i64) -> String {
        self.cat_income
            .iter()
            .chain(self.cat_expense.iter())
            .find(|c| c.id == id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Sem categoria".into())
    }

    pub fn category_color(&self, id: i64) -> Color32 {
        self.cat_income
            .iter()
            .chain(self.cat_expense.iter())
            .find(|c| c.id == id)
            .map(|c| forms::hex_color(&c.color))
            .unwrap_or(Color32::GRAY)
    }

    pub fn account_name(&self, id: i64) -> String {
        self.accounts
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Conta removida".into())
    }

    pub fn total_balance(&self) -> i64 {
        self.accounts.iter().map(|a| account_balance(a, &self.txs)).sum()
    }

    pub fn new_tx(&mut self, kind: TxKind) {
        let account = self.accounts.first().map(|a| a.id).unwrap_or(0);
        let category = match kind {
            TxKind::Income => self.cat_income.first().map(|c| c.id).unwrap_or(0),
            TxKind::Expense => self.cat_expense.first().map(|c| c.id).unwrap_or(0),
            TxKind::Transfer => 0,
        };
        self.form = Some(forms::Form::Tx(forms::TxDraft::new(
            kind,
            account,
            category,
            &self.month,
        )));
    }

    pub fn ask_delete(&mut self, title: &str, message: String, target: Target) {
        self.confirm = Some(Confirm {
            title: title.to_string(),
            message,
            target,
        });
    }

    fn run_delete(&mut self, target: Target) {
        let result = match target {
            Target::Tx(id) => self.db.transaction_delete(id),
            Target::Account(id) => self.db.account_delete(id),
            Target::Goal(id) => self.db.goal_delete(id),
            Target::Category(id) => self.db.category_delete(id),
            Target::Budget(id) => self.db.budget_delete(id),
        };
        match result {
            Ok(()) => {
                self.dirty = true;
                self.budgets = self.db.budget_list(&self.month).unwrap_or_default();
                self.toast("Registro excluído");
            }
            Err(e) => self.toast(format!("Erro ao excluir: {e}")),
        }
    }

    fn nav(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("nav")
            .exact_width(208.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("GranaFlow").size(22.0).strong().color(ACCENT));
                });
                ui.label(
                    RichText::new("finanças pessoais · local-first")
                        .size(10.5)
                        .weak(),
                );
                ui.add_space(18.0);
                for p in Page::ALL {
                    let selected = self.page == p;
                    let fill = if selected {
                        Color32::from_rgba_unmultiplied(16, 185, 129, 34)
                    } else {
                        Color32::TRANSPARENT
                    };
                    let text = if selected {
                        ACCENT
                    } else {
                        ui.visuals().text_color()
                    };
                    let btn = egui::Button::new(RichText::new(p.label()).size(13.5).color(text))
                        .fill(fill)
                        .stroke(egui::Stroke::NONE)
                        .min_size(egui::vec2(178.0, 34.0));
                    if ui.add(btn).clicked() {
                        self.page = p;
                    }
                    ui.add_space(2.0);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(14.0);
                    let worth = net_worth(&self.accounts, &self.txs);
                    ui.label(
                        RichText::new(format!("Patrimônio {}", fmt_brl(worth.net)))
                            .size(12.0)
                            .strong()
                            .color(if worth.net >= 0 { ACCENT } else { EXPENSE }),
                    );
                    ui.label(RichText::new("ativos − dívidas").size(10.0).weak());
                    ui.separator();
                });
            });
    }

    fn confirm_window(&mut self, ctx: &egui::Context) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };
        let mut act = 0u8;
        egui::Window::new(&confirm.title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label(&confirm.message);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(RichText::new("Excluir").color(Color32::WHITE)).fill(EXPENSE))
                        .clicked()
                    {
                        act = 1;
                    }
                    if ui.button("Cancelar").clicked() {
                        act = 2;
                    }
                });
            });
        match act {
            1 => self.run_delete(confirm.target),
            2 => {}
            _ => self.confirm = Some(confirm),
        }
    }

    fn toast_window(&mut self, ctx: &egui::Context) {
        let Some(t) = &self.toast else { return };
        if t.at.elapsed().as_secs_f32() > 3.5 {
            self.toast = None;
            return;
        }
        egui::Area::new(egui::Id::new("toast"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-18.0, -18.0])
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(Color32::from_rgb(20, 30, 48))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(&t.msg).color(Color32::WHITE));
                    });
            });
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.dirty {
            self.reload();
            self.dirty = false;
        }
        self.nav(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(10.0);
                    match self.page {
                        Page::Dashboard => pages::dashboard(self, ui),
                        Page::Transactions => pages::transactions(self, ui),
                        Page::Budget => pages::budget(self, ui),
                        Page::Goals => pages::goals(self, ui),
                        Page::Accounts => pages::accounts(self, ui),
                        Page::Reports => pages::reports(self, ui),
                        Page::Forecast => pages::forecast(self, ui),
                        Page::Settings => pages::settings(self, ui),
                    }
                    ui.add_space(20.0);
                });
        });
        forms::show(self, ctx);
        self.confirm_window(ctx);
        self.toast_window(ctx);
    }
}

pub fn apply_theme(ctx: &egui::Context, dark: bool) {
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    if dark {
        v.panel_fill = Color32::from_rgb(9, 14, 26);
        v.window_fill = Color32::from_rgb(17, 26, 43);
        v.extreme_bg_color = Color32::from_rgb(12, 19, 33);
        v.faint_bg_color = Color32::from_rgb(16, 25, 41);
        v.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 36, 58);
        v.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(34, 48, 74);
        v.widgets.inactive.bg_fill = Color32::from_rgb(24, 36, 58);
        v.selection.bg_fill = Color32::from_rgba_unmultiplied(16, 185, 129, 70);
    } else {
        v.panel_fill = Color32::from_rgb(244, 246, 251);
        v.window_fill = Color32::WHITE;
        v.extreme_bg_color = Color32::WHITE;
        v.faint_bg_color = Color32::WHITE;
        v.widgets.noninteractive.bg_fill = Color32::from_rgb(238, 241, 247);
        v.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(226, 232, 240);
        v.widgets.inactive.bg_fill = Color32::from_rgb(238, 241, 247);
        v.selection.bg_fill = Color32::from_rgba_unmultiplied(16, 185, 129, 50);
    }
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(9.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(13.5),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(13.5),
    );
    ctx.set_style(style);
}

// ---------- helpers visuais usados pelas páginas ----------

pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let fill = ui.visuals().faint_bg_color;
    let stroke = egui::Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color);
    egui::Frame::none()
        .fill(fill)
        .stroke(stroke)
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, add)
        .inner
}

pub fn kpi(ui: &mut egui::Ui, label: &str, value: i64, sub: &str, color: Color32) {
    card(ui, |ui| {
        ui.set_min_height(80.0);
        ui.label(
            RichText::new(label)
                .size(11.0)
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(2.0);
        ui.label(RichText::new(fmt_brl(value)).size(21.0).strong().color(color));
        if !sub.is_empty() {
            ui.label(RichText::new(sub).size(10.5).weak());
        }
    });
}

pub fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(24.0).strong());
    ui.label(RichText::new(subtitle).size(12.0).weak());
    ui.add_space(10.0);
}

pub fn month_bar(ui: &mut egui::Ui, month: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        if ui.button("◀").clicked() {
            *month = shift_month(month.as_str(), -1);
            changed = true;
        }
        ui.add_sized(
            [150.0, 24.0],
            egui::Label::new(RichText::new(month_label(month)).size(15.5).strong()),
        );
        if ui.button("▶").clicked() {
            *month = shift_month(month.as_str(), 1);
            changed = true;
        }
        if ui.small_button("Hoje").clicked() {
            *month = month_key(today());
            changed = true;
        }
    });
    changed
}

pub fn month_label(key: &str) -> String {
    const NAMES: [&str; 12] = [
        "Janeiro", "Fevereiro", "Março", "Abril", "Maio", "Junho", "Julho", "Agosto",
        "Setembro", "Outubro", "Novembro", "Dezembro",
    ];
    let y = key.get(0..4).unwrap_or("?");
    let m: usize = key.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(1);
    format!("{} {}", NAMES[(m.saturating_sub(1)).min(11)], y)
}

pub fn progress(ui: &mut egui::Ui, ratio: f32, color: Color32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect,
        egui::Rounding::same(height / 2.0),
        ui.visuals().widgets.noninteractive.bg_fill,
    );
    let w = rect.width() * ratio.clamp(0.0, 1.0);
    if w > 0.0 {
        painter.rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(w, height)),
            egui::Rounding::same(height / 2.0),
            color,
        );
    }
}


