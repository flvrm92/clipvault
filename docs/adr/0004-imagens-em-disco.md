# ADR-0004 · Imagens em disco, metadados no SQLite

- **Status:** Aceita (2026-11-20) — requisito do produto

## Contexto
Imagens podem ter dezenas de MB; guardá-las como BLOB incha o banco, piora o WAL e o vacuum.

## Decisão
Salvar imagens como **PNG** em `<data>/images/<aa>/<hash>.png` com miniatura `<hash>.thumb.png` (lado maior 320 px). O banco guarda apenas metadados (`image_assets`) e a chave do arquivo (hash do conteúdo). Gravação atômica (tmp → rename → INSERT) e reconciliação na inicialização.

## Alternativas consideradas
- **BLOB no SQLite** — transacional, mas banco pesado e lento para listar.
- **Formato WebP/AVIF** — menor, mas PNG é sem perdas e universal ao recolocar no clipboard.

## Consequências
- \+ Banco pequeno e rápido; miniaturas prontas para a lista.
- − Consistência banco ↔ arquivos precisa de cuidado (REL-002, RET-005).
