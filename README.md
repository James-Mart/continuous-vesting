# Continuous reward vesting

This is a sample implementation of a reward algorithm that causes the reward to continuously vest.

It is intended to achieve the following properties:
* Fixed rate: Each bucket has an immutable per-second vesting rate
* Continuous vesting: Remaining balance decays as P(t)=P₀·e^{−λΔt}
* Claim whenever: The vested portion is claimable at any time
* Deposit whenever: New deposits start vesting immediately from "now"
* Claim-neutrality: Claiming only reduces the unclaimed total; it does not change future vesting
* Deposit-neutrality: Adding rewards does not retroactively change already vested amounts

# How it works

## 1) What we're modeling

* There's an amount $P(t)$ that steadily unlocks over time.
* At any moment you can take the unlocked part; adding more later shouldn't disturb what was already on its way to unlocking.

$deposited = claimed + stillVesting + claimable$

## 2) The core curve

* We model "steady unlocking" with exponential decay:

  $$P(t) = P_0 \cdot e^{-\lambda (t - t_0)}$$

  where:

  * $P_0$ = amount that was still vesting at time $t_0$,
  * $\lambda$ = per-second rate (e.g., $0.01$ for ~1%/s).
* Unlocked by time $t$:

  $$vested(t) = deposited - P(t)$$

**Quick tools**

* Half-life: $T_{1/2}=\ln 2 / \lambda$ (time to cut $P$ in half).
* Exact "1% per second" (multiplicative): $\lambda = -\ln(0.99)$.

## 3) Adding more later

**Rule:** *Snapshot, then add.*
If you add amount $a$ at time $t_d$:

1. First compute the current remainder from the old curve:
   $P_{now} = P_0\,e^{-\lambda (t_d - t_0)}$.
2. Start a fresh curve from "now" with the combined amount:
   new starting amount $= P_{now} + a$, new start time $= t_d$.

**Why this works:** Both old and new parts shrink by the **same percentage per second**. If two things shrink by the same percentage over time, their sum also shrinks by that same percentage. So "snapshot + add" gives the same outcome as tracking each curve separately.

**Mathematical example:**
Suppose you have:
* `f1(t) = A1 * e^(–λ t)`
* `f2(t) = A2 * e^(–λ t)`

Their sum is:
```
f_sum(t) = f1(t) + f2(t) = (A1 + A2) * e^(–λ t)
```

That's just another exponential decay with the same λ. So it behaves identically, just with a different starting value.

## 4) Claiming

* What you can take right now:

  $$claimable(t) = vested(t) - alreadyClaimed$$
* Taking a claim doesn't change the curve for what remains; it only reduces the unclaimed total.


## Calculating decay rate from half-life

It's easier for people to specify a half-life than a decay-rate-in-seconds, so we need a formula for converting a half-life into a decay rate.


If we define half-life as follows:

$$P(T_{1/2})=\tfrac{1}{2}P_0$$

We can derive the formula algebraically:

1. The remaining principal: $$P(t)=P_0\,e^{-\lambda t}.$$
2. Substitute in half-life: $$P_0 e^{-\lambda T_{1/2}}=\tfrac{1}{2}P_0$$
3. Divide both sides: $$e^{-\lambda T_{1/2}}=\tfrac{1}{2}$$
4. Take natural logs: $$-\lambda T_{1/2}=\ln\!\left(\tfrac{1}{2}\right)$$
5. $\ln(1/2)=-\ln 2$, so it reduces to $$-\lambda T_{1/2}=-\ln 2$$
6. Solve for $\lambda$: $$\lambda=\frac{\ln 2}{T_{1/2}}$$
7. If $T_{1/2}$ is given in **days** $d$ and we want $\lambda$ **per second**: $$T_{1/2}=d\cdot 86{,}400 \quad\Rightarrow\quad\lambda=\frac{\ln 2}{d\cdot 86{,}400}$$
