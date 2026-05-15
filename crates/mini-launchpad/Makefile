# Минимум команд для запуска проекта

.PHONY: install validator validator-metaplex sync-program-keys build deploy deploy-devnet deploy-oracle-devnet deploy-minter-devnet init init-devnet mint mint-metaplex mint-devnet deploy-oracle deploy-minter backend backend-devnet frontend kill-frontend test

PROGRAM_KEYPAIR_DIR := program/deploy-keypairs

install:
	cd program && yarn install
	cd frontend && npm install

# Обычный локальный валидатор (без Metaplex)
validator:
	solana-test-validator

# Валидатор с клоном Metaplex Token Metadata (для отображения имени/тикера/картинки в кошельке)
validator-metaplex:
	solana-test-validator --clone-upgradeable-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s --url https://api.mainnet-beta.solana.com

sync-program-keys:
	mkdir -p program/target/deploy
	cp $(PROGRAM_KEYPAIR_DIR)/*-keypair.json program/target/deploy/

build: sync-program-keys
	cd program && anchor build

# Деплоит оба контракта на localnet (сначала запусти make validator в другом терминале)
deploy: build
	cd program && anchor deploy --provider.cluster localnet

# Devnet: переключись (solana config set-url devnet), пополни (solana airdrop 2), затем деплой обоих контрактов:
deploy-devnet: deploy-oracle-devnet deploy-minter-devnet

# Devnet: деплой только оракула или только минтера
deploy-oracle-devnet: build
	cd program && anchor deploy --program-name sol_usd_oracle --provider.cluster devnet

deploy-minter-devnet: build
	cd program && anchor deploy --program-name token_minter --provider.cluster devnet

# Инициализация оракула и минтера (localnet по умолчанию)
init:
	cd program && node scripts/init-local.js

# Инициализация на devnet (после make deploy-devnet)
init-devnet:
	cd program && RPC_URL=https://api.devnet.solana.com node scripts/init-local.js

# Минт без Metaplex metadata (localnet по умолчанию)
mint:
	cd program && node scripts/mint-local.js

# Минт с Metaplex metadata; для localnet сначала запусти make validator-metaplex
mint-metaplex:
	cd program && TOKEN_NAME="Local Metadata Token" TOKEN_SYMBOL=LMT TOKEN_URI=https://example.com/token.json node scripts/mint-local.js

# Минт на devnet после deploy-devnet/init-devnet
mint-devnet:
	cd program && RPC_URL=https://api.devnet.solana.com TOKEN_NAME="Devnet Metadata Token" TOKEN_SYMBOL=DMT TOKEN_URI=https://example.com/token.json node scripts/mint-local.js

# Localnet: деплой только оракула или только минтера (сначала make validator)
deploy-oracle: build
	cd program && anchor deploy --program-name sol_usd_oracle --provider.cluster localnet

deploy-minter: build
	cd program && anchor deploy --program-name token_minter --provider.cluster localnet

# Backend для localnet (читает backend/.env; нужны SOLANA_RPC_HTTP, SOLANA_RPC_WS)
backend:
	cd backend && cargo run

# Backend для devnet (подставляет RPC devnet; ORACLE_STATE_PUBKEY и т.д. из .env)
backend-devnet:
	cd backend && SOLANA_RPC_HTTP=https://api.devnet.solana.com SOLANA_RPC_WS=wss://api.devnet.solana.com cargo run

# Освободить порт 7001 (если занят старым процессом фронта)
kill-frontend:
	-lsof -ti:7001 | xargs kill 2>/dev/null || true

frontend: kill-frontend
	cd frontend && npm run dev

test: build
	cd program && yarn run ts-mocha -p ./tsconfig.json -t 1000000 "tests/**/*.ts"
