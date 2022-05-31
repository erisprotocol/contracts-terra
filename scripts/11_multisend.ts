import yargs from "yargs/yargs";
import { MsgSend } from "@terra-money/terra.js";
import * as keystore from "./keystore";
import { createLCDClient, createWallet, sendTxWithConfirm } from "./helpers";
import * as fs from "fs";

const argv = yargs(process.argv)
  .options({
    network: {
      type: "string",
      demandOption: true,
    },
    key: {
      type: "string",
      demandOption: true,
    },
    "key-dir": {
      type: "string",
      demandOption: false,
      default: keystore.DEFAULT_KEY_DIR,
    },
    "csv-file": {
      type: "string",
      demandOption: true,
    },
    "sender-address": {
      type: "string",
      demandOption: true,
    },
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const worker = await createWallet(terra, argv["key"], argv["key-dir"]);

  const csv = fs
    .readFileSync(argv["csv-file"], "ascii")
    .split("\n")
    .map((a) => {
      const parts = a.split(";");
      return {
        address: parts[0],
        amount: +parts[1],
      };
    })
    .filter((a) => a.amount > 0);

  const { txhash } = await sendTxWithConfirm(
    worker,
    csv.map(
      (a) => new MsgSend(argv.senderAddress, a.address, { uluna: a.amount * 2 })
    )
  );
  console.log(`Success! Txhash: ${txhash}`);
})();
