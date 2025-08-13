# Continuous reward vesting

This is a sample implementation of a reward algorithm that causes the reward to continuously vest.

It is intended to achieve the following properties:
* Fixed rate: Each bucket has an immutable per-second vesting rate
* Continuous vesting: Remaining balance decays as P(t)=P₀·e^{−λΔt}
* Claim whenever: The vested portion is claimable at any time
* Deposit whenever: New deposits start vesting immediately from “now”
* Claim-neutrality: Claiming only reduces the unclaimed total; it does not change future vesting
* Deposit-neutrality: Adding rewards does not retroactively change already vested amounts
