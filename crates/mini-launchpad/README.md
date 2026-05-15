# Solana Mini Launchpad

Учебный мини-лаунчпад на Solana + Anchor: два on-chain контракта (SOL/USD oracle и token minter), Rust backend для обновления цены и прослушки событий, а также Remix фронтенд (папка `frontend/`).

## Структура
- `program/` — Anchor workspace  
  - `programs/sol_usd_oracle` — хранит цену SOL/USD (decimals = 6)  
  - `programs/token_minter` — минтит SPL токены за комиссию в SOL, используя цену из oracle  
  - `tests/` — Anchor TS тесты  
- `backend/` — Rust сервис, который обновляет цену и слушает события `TokenCreated`
- `frontend/` — Remix-приложение для минта токенов через кошелёк

## Быстрый старт (локально)

1. **Validator**: запустить `solana-test-validator` (или `make validator`). Для отображения имени, тикера и картинки токена в кошельке используйте валидатор с клоном Metaplex: `make validator-metaplex` (клон программы Token Metadata с mainnet). Убедитесь, что `~/.config/solana/id.json` есть и профинансирован (`solana airdrop 1000` при необходимости).

2. **Программы**: собрать и задеплоить. ID программ закреплены в `program/deploy-keypairs/` и перед сборкой копируются в `program/target/deploy/`:
   ```bash
   make build
   make deploy
   ```

3. **Инициализация**: один раз после деплоя инициализировать oracle и minter (скрипт выведет `ORACLE_STATE_PUBKEY` для `.env`):
   ```bash
   make init
   ```

4. **Проверочный mint через CLI**:
   ```bash
   make mint
   ```

   Для проверки Metaplex metadata на localnet запустите валидатор через `make validator-metaplex`, затем:
   ```bash
   make deploy
   make init
   make mint-metaplex
   ```

## Деплой на Devnet

На фронте есть переключатель **Localnet / Devnet**. Для тестов на devnet:

1. Пополнить deploy-кошелёк Devnet:
   ```bash
   solana balance --url https://api.devnet.solana.com program/target/devnet/id.json
   ```
   Для чистого деплоя двух программ нужно примерно 5 Devnet SOL. Если CLI-аирдроп
   упирается в rate limit, используйте https://faucet.solana.com/ с GitHub-login или
   альтернативный faucet. Адрес deploy-кошелька: `4GJEK5HnFtpzoKHmPJZ8ab5jqr5FhUp1XgoDThGPoqmo`.

2. Собрать и задеплоить на devnet:
   ```bash
   make deploy-devnet
   ```
   По умолчанию Makefile использует `program/target/devnet/id.json` и передаёт
   `--use-rpc --max-sign-attempts 10`, чтобы write-транзакции деплоя шли через RPC.

3. Инициализировать оракул и минтер на devnet (один раз):
   ```bash
   make init-devnet
   ```

4. Сделать проверочный mint:
   ```bash
   make mint-devnet
   ```

5. В приложении выбрать сеть **Devnet**, в кошельке переключиться на Devnet — можно минтить. На devnet Metaplex уже есть, картинка в кошельке может отображаться (если URI доступен по HTTPS).

6. **Backend**: скопировать `backend/.env.example` в `backend/.env`, подставить `ORACLE_STATE_PUBKEY` из вывода init-скрипта. Путь `BACKEND_KEYPAIR_PATH` поддерживает `~`:
   ```bash
   cd backend
   cargo run
   ```
   Сервис будет периодически вызывать `update_price` и слушать события `TokenCreated`, выводя их в stdout в JSON.

7. **Фронтенд** (опционально):
   ```bash
   cd frontend
   npm install && npm run dev
   ```
  Открыть http://localhost:7001.

8. **Тесты** (LiteSVM, без сети):
   ```bash
   cd program
   anchor test
   ```
   Или `yarn litesvm` для запуска только тестов в `tests/*.litesvm.ts`.

## Переменные окружения для backend

См. `backend/.env.example`. Основные:
- `SOLANA_RPC_HTTP`, `SOLANA_RPC_WS` — RPC локального валидатора или devnet/mainnet.
- `ORACLE_PROGRAM_ID`, `MINTER_PROGRAM_ID` — из `anchor keys list` (после деплоя).
- `ORACLE_STATE_PUBKEY` — PDA от seed `"oracle_state"`; выводится скриптом `program/scripts/init-local.js`.
- `BACKEND_KEYPAIR_PATH` — keypair администратора оракула (поддерживается `~`).
- Опционально: `MOCK_PRICE`, `PRICE_API_URL`, `PRICE_POLL_INTERVAL_SEC`.

## Program IDs

- `sol_usd_oracle`: `3SyERpqhcx5V7z1wc8pwpCftbNhVPCfcW1BNtM5baUm8`
- `token_minter`: `95S6rgKz3RGSwgLiSa7sFkW5iY68TLCzj5ezyNMvQrCc`
- `ORACLE_STATE_PUBKEY`: `34Z57smo7zcABuf5Za3KnHChK2QWLSLCSf3obsx9p2gU`

## Проверенный Devnet deploy

Cluster: Solana Devnet. Deploy wallet / upgrade authority:
`4GJEK5HnFtpzoKHmPJZ8ab5jqr5FhUp1XgoDThGPoqmo`.

- Program ID оракула (`sol_usd_oracle`): `3SyERpqhcx5V7z1wc8pwpCftbNhVPCfcW1BNtM5baUm8`
  ([Solscan](https://solscan.io/account/3SyERpqhcx5V7z1wc8pwpCftbNhVPCfcW1BNtM5baUm8?cluster=devnet))
- Program ID launchpad (`token_minter`): `95S6rgKz3RGSwgLiSa7sFkW5iY68TLCzj5ezyNMvQrCc`
  ([Solscan](https://solscan.io/account/95S6rgKz3RGSwgLiSa7sFkW5iY68TLCzj5ezyNMvQrCc?cluster=devnet))
- PDA оракула (`ORACLE_STATE_PUBKEY`): `34Z57smo7zcABuf5Za3KnHChK2QWLSLCSf3obsx9p2gU`
  ([Solscan](https://solscan.io/account/34Z57smo7zcABuf5Za3KnHChK2QWLSLCSf3obsx9p2gU?cluster=devnet))

Deploy transactions:

- Oracle deploy:
  [`36QstCLPcrAPCD7wfYgBc3LnCJjpi6hE4QEjnamSy6myuTakxKWmpi4rpxPrqarAfraBQDKF8E54Vyv8BdPbCucw`](https://solscan.io/tx/36QstCLPcrAPCD7wfYgBc3LnCJjpi6hE4QEjnamSy6myuTakxKWmpi4rpxPrqarAfraBQDKF8E54Vyv8BdPbCucw?cluster=devnet)
- Launchpad deploy:
  [`2CYGj9KR7qkmKCoe9sf6JddSUj7FjqhWm4BNo4cRRycTKaGy4a8Aw3Un7ANoW1vKHgmxLjUpzo3jLBKKrUA9mDvU`](https://solscan.io/tx/2CYGj9KR7qkmKCoe9sf6JddSUj7FjqhWm4BNo4cRRycTKaGy4a8Aw3Un7ANoW1vKHgmxLjUpzo3jLBKKrUA9mDvU?cluster=devnet)

Initialization transactions:

- Initialize oracle:
  [`dR2Cm514AD4MXkBckz41xnaWa3ew5wYakxYXXoA3K5ZrhRkjAx6hRemJGTyprZrbJ6f1XsvJMf8cC2ydH8U5fTW`](https://solscan.io/tx/dR2Cm514AD4MXkBckz41xnaWa3ew5wYakxYXXoA3K5ZrhRkjAx6hRemJGTyprZrbJ6f1XsvJMf8cC2ydH8U5fTW?cluster=devnet)
- Set initial price:
  [`54mkcusrj8hBvAG4vNVb7P93tU5o4kqKowPGckGmZAeYbJobnF3aXHecJMXEs4JeAsKqRguC1QZF2ddVGnPvY45H`](https://solscan.io/tx/54mkcusrj8hBvAG4vNVb7P93tU5o4kqKowPGckGmZAeYbJobnF3aXHecJMXEs4JeAsKqRguC1QZF2ddVGnPvY45H?cluster=devnet)
- Initialize launchpad:
  [`4Ry1yHDNpG87TNkPc2LtYfpzf1PuaXdvemyfxWopM4fsGM6ikwwu8dDPqSY5nLpzPNDe4TvhXvNqFSrcr2f7nrAs`](https://solscan.io/tx/4Ry1yHDNpG87TNkPc2LtYfpzf1PuaXdvemyfxWopM4fsGM6ikwwu8dDPqSY5nLpzPNDe4TvhXvNqFSrcr2f7nrAs?cluster=devnet)

Mint transactions on Devnet:

- Mint 1:
  [`3skuUwr6xyHyEzghqtUM3H3TvQHznbU7xg9aST1ufDCCEsMAJn8rE17JdWsaA4ya3UGaVs1D8JDBoeTPkyXmCsnN`](https://solscan.io/tx/3skuUwr6xyHyEzghqtUM3H3TvQHznbU7xg9aST1ufDCCEsMAJn8rE17JdWsaA4ya3UGaVs1D8JDBoeTPkyXmCsnN?cluster=devnet),
  mint `VLbuMMGn7uoTmC1A3b8Wg3GTd47SBxEHWzVPNGTxEAL`
- Mint 2:
  [`2iTvfHseMKYRB8fsC3m7NcnjH8MHvL57nu8HRwMCtHhNEEYkFEri83JzJQj9QYRhxc6rgjMgUWZLhANSihVvs6V`](https://solscan.io/tx/2iTvfHseMKYRB8fsC3m7NcnjH8MHvL57nu8HRwMCtHhNEEYkFEri83JzJQj9QYRhxc6rgjMgUWZLhANSihVvs6V?cluster=devnet),
  mint `ERsJWzBwQLzzXL9mMD4q2NdRg7J9wNm3U94TEpriMS2E`
- Mint 3:
  [`4CJjTHV2QuxEPibJ4HiPoPRJWnBdFajWWQ7GmNVsatbgt4J3fDGKbro2CsrgR9f7QqM6jgAo2wdUoGFSy4ega5ur`](https://solscan.io/tx/4CJjTHV2QuxEPibJ4HiPoPRJWnBdFajWWQ7GmNVsatbgt4J3fDGKbro2CsrgR9f7QqM6jgAo2wdUoGFSy4ega5ur?cluster=devnet),
  mint `4mEzMUb5v32TfeMwf7mKdzSojdeQTbEdUcPLqsRK9jhJ`

## Проверенный localnet metadata mint

Local validator with cloned Metaplex Token Metadata:

- Mint signature: `4zyPPjHfXWFVAJbwxmtVqPVkAscnai7stKEt4hrpDwxeJjuJXnUL4M6N1Xh2sfDSz7u4Vnsx3x3xNMz89YBM8gAr`
- Mint: `E6XraTF8zs55ijbfBGSUe3L72PDPPewT11m1AA2e7A1w`
- Metadata PDA: `Bm9Muu4iKQevFp7pcbUwuAws6fKcWAcpFRBMe11EATGC`
- Metadata owner: `metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s`

## Метаданные токена (Metaplex)

При минте можно передать `name`, `symbol` и `uri` — контракт создаёт запись Metaplex Token Metadata (имя, тикер, картинка в кошельке). Если передать пустое имя, метаданные не создаются (подходит для localnet без Metaplex). Для отображения в кошельке поднимайте валидатор с клоном Metaplex: `make validator-metaplex`, затем деплой и `init` как обычно.

## Основные ограничения
- Все вычисления комиссии — integer math, `fee_lamports = mint_fee_usd * LAMPORTS_PER_SOL / price`.
- Oracle price и mint_fee_usd хранятся с точностью 10^6.
- Доступ к `update_price` только у oracle admin (backend keypair).
- `mint_token` падает, если `price == 0` или fee/supply некорректны.


---

## Порядок запуска (локально)

1. `solana-test-validator`
2. `cd program && anchor build && anchor deploy --provider.cluster localnet`
3. `cd program && node scripts/init-local.js` — скопировать `ORACLE_STATE_PUBKEY` в `backend/.env`
4. `cd backend && cargo run`
5. `cd frontend && npm run dev` — открыть в браузере и покликать.
