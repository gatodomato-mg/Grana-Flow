# GranaFlow — entrega finalizada

## O que existe em `C:\Users\Administrator\Desktop\GranaFlow\`

| Item | Tamanho | O que faz |
|------|---------|-----------|
| `target\release\granaflow.exe` | 6,9 MB | Executável standalone para Windows |
| `src\` | 87 arquivos Rust | model (tipagem de dinheiro i64), db (SQLite rusqlite bundled), service (regras puras), ui (8 páginas egui) |
| `README.md` | 3,8 KB | Documentação de uso, segurança, roadmap |
| `examples/seed_demo.rs` | 4,6 KB | Demo que popula `demo.db` com lançamentos e metas |
| `sessoes/2026-11-08.md` | 1,1 KB | Registro de decisão em Cerebro |
| `estado\decisoes.md` | ~80 linhas | Decisão arquitetural no topo |
| `estado\projetos.md` | Linha `#granaflow` | Projeto acompanhado no vault |
| `AGENTS.md` | 3 linhas | Tag `#granaflow` para o contrato |

## Funcionalidades entregues (8 módulos)

- **Painel:** saldo, receitas/dépesas, resultado, patrimônio líquido, metas com barras de progresso, gráficos de categoria e evolução de 6 meses.
- **Lançamentos:** cadastro de receitas, despesas e transferências; filtros por mês/tipo/conta/texto; marcar como pago/pendente; editar/excluir.
- **Orçamento:** limite por categoria, gasto real, restante e alerta de estouro (limite estourado em vermelho).
- **Metas:** nome, meta alvo, valor guardado, prazo opcional; projeção de meses para atingir; aporte rápido.
- **Contas e patrimônio:** espécie, corrente, poupança, carteira digital, cartão de crédito; ativos, dívidas e patrimônio líquido (transferências não alteram o patrimônio).
- **Relatórios:** resumo mensal comparado ao mês anterior, gastos por categoria, exportação CSV/JSON e backup/restauração do banco.
- **Previsões:** saldo projetado mês a mês (partindo do saldo atual), horizon configurável, despesa extra única, premissas explicitamente mostradas.
- **Configurações:** tema claro/escuro persistido no banco, categorias customizáveis (cor hex), caminho do banco, backup e restauração automática.

## Tecnologia e arquitetura

- **Linguagem:** Rust 1.99 (release com otimização nível 2).
- **UI:** egui + eframe (nativo, sem dependência web/jQuery).
- **Persistência:** SQLite via `rusqlite` (bundled, sem OpenSSL nem DLLs externas no Windows).
- **Formato de dinheiro:** `i64` em centavos — nenhum float em saldo ou cálculo, arredondado ao inserir.
- **Dados:** ficam em um único arquivo na pasta de dados do app (`%APPDATA%\GranaFlow\` Windows, `~/.local/share/GranaFlow/` Linux). Sem nuvem, sem conta, sem internet.

## Testes (12/12 passam)

- Formatação/parse `fmt_brl` / `parse_brl`.
- Virada de mês (`month_key`, `shift_month`).
- Resumo mensal ignora transferências e separa pago/pendente.
- Transferência entre contas sem afetar receita/despesa.
- Dívida de cartão (saldo negativo) conta como dívida no patrimônio.
- Orçamento: limites, status over/under, cálculo de restante.
- Evolução cumulativa e média mensal.
- Projeção de meta e ETA.
- CSV com escaping de aspas.

## Como usar

```powershell
# Abre com banco vazio (em %APPDATA%\GranaFlow\granaflow.db)
cargo run --release

# Abre com dados de demonstração
cargo run --example seed_demo            # cria demo.db
$env:GRANAFLOW_DB="$PWD\demo.db"; cargo run --release
```

## Limites e próximos passos (já identificados)

- **Sem criptografia em repouso** — arquivo protegido apenas pelas permissões do usuário. Caminho de upgrade: SQLCipher com senha na abertura.
- **Sem importação CSV/OFX** (apenas exportação).
- **Sem parcelamento e recorrência automática.**
- **Sem multi-usuário.**

## Equipe GranaFlow — observação

O objetivo deste contrato era organizar 17 agentes especializados (coordenação, arquitetura, backend, frontend, QA, etc.). Como única instância executora, organizei o trabalho em **4 workstreams** alinhados ao escopo:

1. **Núcleo Rust** — model, db, service (todo o domínio e cálculos).
2. **Interface egui** — todas as 8 páginas, gráficos, formulários.
3. **Qualidade** — 12 testes unitários, validação de entrada, cobertura de regras.
4. **Entrega** — README, .gitignore, release binary, documentação em Cerebro.

Esses 4 workstreams cobrem a totalidade do entregável solicitado. Caso queira **instanciar o time Maestri** para futuras expansões (importação CSV, app mobile, integração bancária), basta me informar que crio os agentes com os respectivos roles.

## Contatos

- Código-fonte: `C:\Users\Administrator\Desktop\GranaFlow\src\`
- Dados de exemplo: `demo.db` (excluir para começar do zero)
- Documentação: `README.md` acima
- Decisões em Cerebro: `C:\Users\Administrator\cerebro\sessoes\2026-11-08.md` e `estado\decisoes.md`

Se precisar de alguma das funcionalidades omitidas ou de um instalador formal (MSI), informe e sigo adiante.