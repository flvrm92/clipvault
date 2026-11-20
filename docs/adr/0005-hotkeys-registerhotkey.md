# ADR-0005 · Atalhos globais com RegisterHotKey próprio

- **Status:** Aceita (2026-11-20)

## Contexto
O app registra um número variável de atalhos (abertura + N snippets), precisa saber exatamente qual registro falhou por conflito com outro app (HK-004), suspender todos durante a gravação de atalho na UI e ser testável via porta.

## Decisão
Implementar `HotkeyRegistrar` com `RegisterHotKey`/`UnregisterHotKey` numa thread dedicada com loop de mensagens (padrão *Active Object*, P-13), com `MOD_NOREPEAT`.

## Alternativas consideradas
- **tauri-plugin-global-shortcut** — pronto, mas acopla atalhos ao Tauri (fora da porta), com menos controle de erro por atalho e de suspensão.
- **Low-level keyboard hook (`WH_KEYBOARD_LL`)** — permite qualquer combinação (inclusive reservadas), mas é mais invasivo, impacta latência de digitação do sistema e levanta preocupações de privacidade/certificação.

## Consequências
- \+ Controle total e testes de contrato.
- − Combinações já reservadas pelo sistema (ex.: `Win+V`) não podem ser usadas — coerente com HK-003.
