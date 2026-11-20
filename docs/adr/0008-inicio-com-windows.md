# ADR-0008 · Início com o Windows: StartupTask com fallback na chave Run

- **Status:** Aceita (2026-11-20)

## Contexto
CFG-003 pede "Iniciar com o Windows" via `StartupTask` do MSIX. A API `Windows.ApplicationModel.StartupTask` só funciona em processos com identidade de pacote. Até o M7 (e sempre em builds de desenvolvimento) o ClipVault roda sem pacote, então a opção ficaria impossível de usar e de testar de verdade.

## Decisão
A porta `StartupManager` (camada `application`) tem dois adaptadores na infraestrutura, escolhidos na inicialização:

- **Com identidade de pacote** (`GetCurrentPackageFullName` bem-sucedido): `StartupTask` WinRT com `TaskId="ClipVaultStartup"`. Habilitar chama `RequestEnableAsync`; desabilitar chama `Disable`. Os estados `DisabledByUser`, `DisabledByPolicy` e `EnabledByPolicy` são repassados como estão.
- **Sem pacote**: valor `ClipVault` na chave `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, apontando para o executável atual entre aspas. Se o usuário desligou o app no Gerenciador de Tarefas (`…\Explorer\StartupApproved\Run`, primeiro byte ímpar), o estado é `DisabledByUser` e o app não reativa sozinho, como no MSIX.

As chamadas WinRT bloqueiam (`.get()`), então os comandos Tauri correspondentes são assíncronos e rodam em thread de bloqueio, nunca na thread principal (STA).

## Alternativas consideradas
- **Só StartupTask** — segue a spec ao pé da letra, mas a opção fica desabilitada até o M7 e não dá para validar o fluxo antes do empacotamento.
- **Agendador de Tarefas** — mais permissões e mais estado para limpar; sem vantagem sobre a chave Run para um app por usuário.
- **`tauri-plugin-autostart`** — usa a chave Run, mas não conhece `StartupTask` nem o `StartupApproved`; teríamos dois mecanismos de qualquer forma.

## Consequências
- \+ A opção funciona em desenvolvimento e em builds sem pacote; o comportamento ("desligado pelo usuário" não é reativado) é o mesmo nos dois modos.
- \+ O caso de uso não sabe qual adaptador está ativo; os testes usam um fake.
- − Dois adaptadores para manter. O da chave Run é coberto por testes contra uma subchave temporária; o da StartupTask só é verificável empacotado (checklist manual do M7).
- − Quem instalar uma build sem pacote e depois a da Store pode ficar com as duas entradas; a versão empacotada remove o valor da chave Run na inicialização.
