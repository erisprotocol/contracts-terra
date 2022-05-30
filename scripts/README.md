# Scripts

This directory contains scripts to deploy, migrate, or interact with Eris Stake Hub smart contract.

## How to Use

Install dependencies:

```bash
cd scripts
yarn
```

Import the key to use to sign transactions. You will be prompted to enter the seed phrase and a password to encrypt the private key. By default, the encrypted key will be saved at `scripts/keys/{keyname}.json`. The script also provide commands to list or remove keys.

```bash
ts-node 1_manage_keys.ts add <keyname> [--key-dir string]
```

To deploy the contract, create a JSON file containing the instantiation message, and use the following command. You will be prompted to enter the password to decrypt the private key.

```bash
ts-node 2_deploy.ts \
  --network mainnet|testnet|localterra \
  --key keyname \
  --msg /path/to/instantiate_msg.json
```

To stake Luna and mint Stake:

```bash
ts-node 4_bond.ts \
  --network mainnet|testnet|localterra \
  --key keyname \
  --contract-address terra... \
  --amount 1000000
```

Other scripts work similarly to the examples above.


## Real examples

```bash
ts-node 1_manage_keys.ts add mainnet
```

### Testnet
```bash
ts-node 2_deploy.ts --network testnet --key testnet --hub-code-id 126 --token-code-id 125
```

```bash
ts-node 3_migrate.ts --network testnet --key testnet --contract-address terra1cgurv08h780ygh3a4l2tjtxndksywskxp4mypkazuuazqas5m8kqleeupz
```

```bash
ts-node 5_harvest.ts --network testnet --key testnet --hub-address terra1cgurv08h780ygh3a4l2tjtxndksywskxp4mypkazuuazqas5m8kqleeupz
```

```bash
ts-node 6_rebalance.ts --network testnet --key testnet --hub-address terra1cgurv08h780ygh3a4l2tjtxndksywskxp4mypkazuuazqas5m8kqleeupz
```

```bash
ts-node 10_add_validator.ts --network testnet --key testnet --hub-address terra1cgurv08h780ygh3a4l2tjtxndksywskxp4mypkazuuazqas5m8kqleeupz --validator-address terravaloper1uxx32m0u5svtvrujnpcs6pxuv7yvn4pjhl0fux
```


### Mainnet
```bash
ts-node 2_deploy.ts --network mainnet --key mainnet --hub-code-id 11 --token-code-id 12
```

```bash
ts-node 3_migrate.ts --network mainnet --key mainnet --contract-address terra10788fkzah89xrdm27zkj5yvhj9x3494lxawzm5qq3vvxcqz2yzaqyd3enk
```

```bash
ts-node 5_harvest.ts --network mainnet --key mainnet --hub-address terra10788fkzah89xrdm27zkj5yvhj9x3494lxawzm5qq3vvxcqz2yzaqyd3enk
```
