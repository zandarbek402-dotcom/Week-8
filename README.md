# Неделя 8 — РК1; BUILD SPRINT: каркас Liquidity Vault

Старт сквозного проекта: **DeFi-vault**, который принимает токены пользователей и (в следующих неделях) размещает их в пуле ликвидности.
На этой неделе собран **каркас vault** на Anchor:

- **аккаунт настроек** (`VaultConfig`, PDA);
- **PDA пользователя** (`UserPosition`);
- **share-токен** (mint-PDA, mint authority = config PDA).

Вопросы РК1 (темы недель 1–7) с ответами — в [PK1.md](PK1.md).

## Структура

```
programs/liquidity_vault/src/
├── lib.rs                  # точка входа, список инструкций
├── constants.rs            # seeds для PDA, лимиты
├── errors.rs               # VaultError
├── state.rs                # VaultConfig, UserPosition
├── math.rs                 # расчёт shares <-> assets (+ unit-тесты)
└── instructions/
    ├── initialize_vault.rs # config + share mint + vault token account
    ├── initialize_user.rs  # PDA пользователя
    ├── deposit.rs          # токены -> vault, mint shares
    ├── withdraw.rs         # burn shares -> токены обратно (минус fee)
    └── update_config.rs    # admin: fee / cap / pause
tests/liquidity_vault.ts    # интеграционные тесты (anchor test)
```

## Аккаунты (PDA)

| Аккаунт | Seeds | Назначение |
|---|---|---|
| `VaultConfig` | `["config", underlying_mint]` | admin, mint, share_mint, fee_bps, deposit_cap, paused, bumps |
| Share mint | `["shares", config]` | SPL-токен долей vault; authority — config PDA |
| Vault token account | `["vault", config]` | хранит депозиты; authority — config PDA |
| `UserPosition` | `["user", config, owner]` | shares и сумма депозита пользователя |

```
 User ──deposit(amount)──► [vault token acc]      (PDA ["vault", config])
   ▲                              │
   │◄──── mint shares ──── [share mint] (authority = config PDA)
   │
 withdraw(shares): burn shares ─► vault отдаёт amount - fee
                                   │
                    (неделя 9+) ──► Liquidity Pool (CPI)
```

## Логика share-токена

- Первый депозит: `shares = amount` (1:1).
- Далее: `shares = amount * total_shares / total_assets`.
- Вывод: `assets = shares * total_assets / total_shares`, комиссия `fee_bps` остаётся в vault (выгода остальным держателям).
- Вся математика через `u128` и `checked_*` — без переполнений.

## Инструкции

| Инструкция | Кто | Что делает |
|---|---|---|
| `initialize_vault(fee_bps, deposit_cap)` | admin | создаёт config, share mint, vault token account |
| `initialize_user()` | пользователь | создаёт `UserPosition` PDA |
| `deposit(amount)` | пользователь | перевод токенов в vault + mint shares |
| `withdraw(shares)` | пользователь | burn shares + перевод токенов пользователю |
| `update_config(fee?, cap?, paused?)` | только admin | изменение настроек / пауза |

Проверки безопасности: `has_one` (admin, share_mint, vault_token_account, underlying_mint), seeds + bump для каждого PDA, `token::mint/authority` для токен-аккаунтов, лимит комиссии 10%, deposit cap, пауза.

## Запуск

Требуется: Rust, Solana CLI 1.18+, Anchor 0.30.1, Node.js.

```bash
npm install
anchor keys sync      # подставит ваш program id
anchor build
anchor test           # поднимет локальный валидатор и запустит тесты
cargo test --lib      # unit-тесты математики shares
```

## Что дальше (неделя 9+)

- поле `pool` в `VaultConfig` → интеграция с пулом ликвидности (CPI `add_liquidity` / `remove_liquidity`);
- учёт доходности пула в `total_assets`;
- Token-2022 / `token_interface`.
