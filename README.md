# GranaFlow

Gerenciador financeiro pessoal **local-first** em Rust. Seus dados ficam em um único
arquivo SQLite na sua máquina — sem conta, sem nuvem, sem internet.

![Rust](https://img.shields.io/badge/Rust-1.99-000?logo=rust) ![egui](https://img.shields.io/badge/UI-egui%2Feframe-10b981) ![SQLite](https://img.shields.io/badge/SQLite-bundled-003b57?logo=sqlite)

## Rodar

```powershell
cd C:\Users\Administrator\Desktop\GranaFlow
cargo run --release
```

O banco fica em `%APPDATA%\GranaFlow\granaflow.db` (Windows) ou
`~/.local/share/GranaFlow/granaflow.db` (Linux). Para usar outro arquivo:

```powershell
$env:GRANAFLOW_DB="C:\caminho\meu.db"; cargo run --release
```

Demonstração com dados de exemplo:

```powershell
cargo run --example seed_demo          # cria demo.db
$env:GRANAFLOW_DB="$PWD\demo.db"; cargo run --release
```

## O que já funciona

| Módulo | Entrega |
| --- | --- |
| Painel | saldo, receitas, despesas, resultado, patrimônio, metas, gráficos de categoria e evolução |
| Lançamentos | receitas, despesas e transferências; filtros por mês/tipo/conta/texto; pagar/pendente; editar e excluir |
| Orçamento | limite por categoria, gasto, restante, alerta de estouro |
| Metas | progresso, aporte, prazo e projeção de quando a meta é atingida |
| Contas e patrimônio | espécie, corrente, poupança, carteira digital, cartão; ativos, dívidas e líquido |
| Relatórios | resumo mensal, comparação com mês anterior, gastos por categoria, exportação CSV/JSON, backup |
| Previsões | saldo projetado mês a mês, despesa extra, ETA das metas, premissas explícitas |
| Configurações | tema claro/escuro, categorias, backup/restauração, caminho do banco |

## Arquitetura

```
src/model.rs     domínio: dinheiro em centavos (i64), datas, formatação pt-BR
src/db.rs        SQLite (rusqlite bundled): schema, seed, CRUD, backup/restore
src/service.rs   regras financeiras puras (testáveis sem banco)
src/ui/          egui: shell, páginas, formulários e gráficos desenhados no painter
examples/        seed_demo (dados de demonstração)
tests            testes de unidade em model/service/db
```

Decisões que sustentam os cálculos:

- dinheiro é sempre `i64` em centavos — nenhum float em saldo;
- transferência entre contas não é receita nem despesa;
- saldo usa lançamentos pagos; orçamento e relatórios usam o mês inteiro;
- cartão de crédito com saldo negativo entra como dívida no patrimônio;
- previsões repetem um resultado mensal e dizem isso na tela.

## Testes

```powershell
cargo test
```

12 testes cobrem formatação/parse de valores, virada de mês, resumo mensal,
transferências, dívida de cartão, orçamento, evolução, metas, previsão e CSV.

## Segurança e privacidade

- 100% local; nenhuma requisição de rede no código.
- Sem senhas bancárias, sem integração bancária.
- Backup gera um SQLite consistente (`VACUUM INTO`); restaurar faz cópia automática antes.
- Entrada validada nos formulários e restrições no schema (`amount_cents > 0`, tipos).
- **Não implementado:** criptografia em repouso (SQLCipher) — o arquivo fica protegido
  apenas pelas permissões do usuário. Caminho de upgrade: trocar `bundled` por
  `bundled-sqlcipher-vendored-openssl` e pedir senha na abertura.

## Limites conhecidos (por escolha)

- Sem importação de CSV/OFX (só exportação).
- Sem PDF (CSV/JSON cobrem o uso atual).
- Sem parcelamento de cartão e sem recorrência automática.
- Sem multi-usuário.

## Próximos passos sugeridos

1. Criptografia opcional do banco (SQLCipher) com senha na abertura.
2. Importação CSV/OFX com prévia antes de gravar.
3. Lançamentos recorrentes e parcelados.
4. Instalador Windows (MSI) e AppImage Linux.
"# Grana-Flow" 
