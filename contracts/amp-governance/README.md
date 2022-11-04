# Amp Governance

This product allows voting in governance based on locked ampLP positions.

## Process

1. User locks ampLP for 1 year
2. User has not yet voted in amp_gauge, so his vote is only creating vAMP
3. User votes in amp_gauges for favorite validators
4. User locks more capital in voting_escrow -> sends update to amp_gauges

### Operator

1. TuneEmps of emp_gauges to create a snapshot for the current period
2. TuneVamp on amp_gauges to create a snapshot for the current period
3. TuneDelegations on hub to calculate delegation for the period, store them and start redelegation.
4. Start redelegation on hub

5. User locks ampLP for 1 year

## License

amp-governance is a fork of <https://github.com/astroport-fi/astroport-governance>. Mainly the voting_escrow and generator_controller smart contract has been integrated to fit our use case.

The license is GPL v3 and compatible with our own GPL v3 license.
