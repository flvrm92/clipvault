# ADR-0003 · SQLite via rusqlite (bundled)

- **Status:** Aceita (2026-11-20)

## Contexto
O armazenamento deve ser SQLite (requisito). Acesso é local, de baixo volume e majoritariamente síncrono a partir de threads de trabalho.

## Decisão
`rusqlite` com feature `bundled` (versão do SQLite controlada e idêntica em todas as máquinas). Migrações por um executor próprio (`infrastructure::sqlite::migrations`): versão em `PRAGMA user_version` e cada migração numa transação junto com a troca de versão. WAL ativado.

**Revisão (2026-11-20, M2):** `rusqlite_migration` foi descartado porque fixa uma versão antiga do `rusqlite` (0.41 contra 0.43), e o SQLite embutido só pode ser linkado uma vez. O executor próprio tem ~20 linhas e elimina esse acoplamento.

**Conexão:** uma única conexão protegida por mutex, compartilhada pelos repositórios. As escritas são raras e curtas (ritmo humano) e o WAL garante a durabilidade. Trocar por um pool só se houver contenção medida.

## Alternativas consideradas
- **sqlx** — async e checagem de queries em compilação, mas exige runtime async e banco para `prepare`; ganho pequeno aqui.
- **Diesel / SeaORM** — ORM desnecessário para ~4 tabelas; mais complexidade.
- **tauri-plugin-sql** — expõe SQL ao frontend, contrário a SEC-002 e à arquitetura.

## Consequências
- \+ Simples, síncrono, sem runtime extra; fácil de testar com arquivos temporários.
- − SQL escrito à mão: mitigado por testes de integração dos repositórios.
- **Ponto de reavaliação:** busca com `LIKE` sobre coluna normalizada; migrar para FTS5 se PERF-004 não for atendido.
