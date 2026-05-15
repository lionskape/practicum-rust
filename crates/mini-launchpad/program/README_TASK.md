# Program Task

Это учебная версия `program/` для студентов. В этой копии задание уже решено.

Что было исправлено:
- `sol_usd_oracle` сохраняет новую цену и слот обновления;
- `token_minter` рассчитывает комиссию в lamports из USD-комиссии и цены SOL/USD;
- LiteSVM-тесты проверяют фактические 6 decimals и правильную формулу комиссии.

Как запускать:

```bash
cd program
yarn install
anchor build
yarn run ts-mocha -p ./tsconfig.json -t 1000000 "tests/**/*.ts"
```

Ожидаемый результат: после `anchor build` все LiteSVM-тесты проходят.
