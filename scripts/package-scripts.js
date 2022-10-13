// const npsUtils = require("nps-utils"); // not required, but handy!

module.exports = {
  scripts: {
    release: {
      default: "bash build_release.sh",
    },
    schema: {
      default:
        "nps schema.create schema.hub schema.token schema.ampextractor schema.farm schema.compound schema.fees schema.generator",

      create: "bash build_schema.sh",

      hub: "cd .. && json2ts -i contracts/hub/**/*.json -o ../liquid-staking-scripts/types/hub",
      token:
        "cd .. && json2ts -i contracts/token/**/*.json -o ../liquid-staking-scripts/types/token",
      ampextractor:
        "cd .. && json2ts -i contracts/amp-extractor/**/*.json -o ../liquid-staking-scripts/types/amp-extractor",

      farm: "cd .. && json2ts -i contracts/amp-compounder/astroport_farm/**/*.json -o ../liquid-staking-scripts/types/amp-compounder/astroport_farm",
      compound:
        "cd .. && json2ts -i contracts/amp-compounder/compound_proxy/**/*.json -o ../liquid-staking-scripts/types/amp-compounder/compound_proxy",
      fees: "cd .. && json2ts -i contracts/amp-compounder/fees_collector/**/*.json -o ../liquid-staking-scripts/types/amp-compounder/fees_collector",
      generator:
        "cd .. && json2ts -i contracts/amp-compounder/generator_proxy/**/*.json -o ../liquid-staking-scripts/types/amp-compounder/generator_proxy",
    },
  },
};
