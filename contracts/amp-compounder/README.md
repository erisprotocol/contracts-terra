# License

amp-compounder is a fork of <https://github.com/spectrumprotocol/spectrum-core>

## Changes

- fees_collector
  - Support sending funds to a smart contract with a predefined message to wake it up.
- astroport_farm
  - Use minting / burning of amp[LP] token instead of tracking reward info locally
  - Query interface changed to include exchange_rate and more info
- compound_proxy
  - Instead of supporting only a single compound LP, supports a list of LPs
  - Supports zapping from any asset to LP.
