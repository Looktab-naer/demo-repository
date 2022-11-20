# NFT Series Implementation

## Instructions
start with
`yarn && yarn test:deploy`

#### Pre-reqs

Need rust, cargo, near-cli, near login.

## Example Call

### Deploy
```
near deploy --accountId testm5.testnet --wasmFile out/main.wasm
```

### NFT init
```
near call testm5.testnet --accountId testm5.testnet new_default_meta '{"owner_id":"testm5.testnet", "treasury_id":"testm5.testnet"}'
```

### NFT create series
```
near call --accountId testm5.testnet testm5.testnet nft_mint '{"token_series_id":"1","receiver_id":"csummer.testnet"}' --depositYocto 6960000000000000000000
```

### NFT mint series (Creator only)
```
env NEAR_ENV=local near call --keyPath ~/.near/localnet/validator_key.json --accountId alice.test.near comic.test.near nft_mint '{"token_series_id":"1","receiver_id":"comic.test.near"}' --depositYocto 11280000000000000000000
```

### NFT transfer
```
near call --accountId csummer.testnet testm7.testnet nft_transfer '{"token_id":"2:1","receiver_id":"testm7.testnet"}' --depositYocto 1
```


### NFT burn
```
near call --accountId csummer.testnet testm7.testnet nft_burn '{"token_id":"1:1"}' --depositYocto 1
```

### NFT tokens for owner
```
near view testm7.testnet nft_tokens_for_owner '{"account_id":"csummer.testnet"}'
near view testm7.testnet nft_token '{"token_id":"1:1"}'
```
