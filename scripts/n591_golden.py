#!/usr/bin/env python3
"""Naryad №591 authoring-time golden reference for the spectral contour.

INDEPENDENT implementation of the contracted algorithm (issue #1016,
the DoD row «второй независимый метод»): the classic Lomb–Scargle
periodogram written FROM THE CONTRACT TEXT in plain Python (no Metalogos
code involved), cross-checked at authoring time against
scipy.signal.lombscargle (a third implementation, C loops).

  Contract (issue #1016 + the module header of src/builtins/spectral.rs):
    - grid: f_min = 1/T, f_max = 1/(2·median positive gap) with a fallback
      to the smallest positive gap, Δf = 1/(N·T)  (N-fold oversampled
      Rayleigh limit);
    - per-frequency phase offset τ = atan2(Σsin 2ωt, Σcos 2ωt)/(2ω);
    - power P(ω) = (yc²/Σcos² + ys²/Σsin²)/(2σ²), σ² = sample variance
      (ddof = 1) about the mean;
    - false alarm of the maximum over M bins: 1 − (1 − e^{−z})^M;
    - power_fraction: the share of the TOTAL power inside ±1/T of the peak.

Run: python3 scripts/n591_golden.py
Output: the pinned numbers (freqs/powers subset + the peak stats) for the
FIXED dataset below, at full f64 precision. The values are embedded in
tests/naryad_591_spectral.rs with rel tolerance 1e-9 (sum-order/libm
headroom). The scipy cross-check verdict is printed at the end.
"""

import math

# The FIXED golden dataset (irregular sampling, mixed periods 6.1 and 2.7).
TIMES = [
    0.0, 0.7, 1.3, 1.9, 2.8, 3.4, 4.1, 4.9, 5.5, 6.3, 7.1, 7.8,
    8.4, 9.2, 9.9, 10.6, 11.4, 12.2, 12.9, 13.6, 14.3, 15.1, 15.8, 16.6,
]


def values_of(times):
    return [
        math.sin(2.0 * math.pi * t / 6.1) + 0.5 * math.cos(2.0 * math.pi * t / 2.7)
        for t in times
    ]


def derive_grid(times):
    t_min, t_max = min(times), max(times)
    baseline = t_max - t_min
    sorted_t = sorted(times)
    gaps = [b - a for a, b in zip(sorted_t, sorted_t[1:]) if b - a > 0.0]
    median_gap = sorted(gaps)[len(gaps) // 2]
    f_min = 1.0 / baseline
    f_max = 1.0 / (2.0 * median_gap)
    if not (f_max > f_min):
        f_max = 1.0 / (2.0 * sorted(gaps)[0])
    n = len(times)
    delta_f = 1.0 / (n * baseline)
    m = int(math.floor((f_max - f_min) / delta_f)) + 1
    return [f_min + k * delta_f for k in range(m)], baseline, m


def ls_power(freq, times, y, variance):
    omega = 2.0 * math.pi * freq
    s2 = sum(math.sin(2.0 * omega * t) for t in times)
    c2 = sum(math.cos(2.0 * omega * t) for t in times)
    tau = 0.5 * math.atan2(s2, c2) / omega
    yc = ys = sc = ss = 0.0
    for i, t in enumerate(times):
        a = omega * (t - tau)
        c, s = math.cos(a), math.sin(a)
        yc += y[i] * c
        ys += y[i] * s
        sc += c * c
        ss += s * s
    if not (sc > 0.0) or not (ss > 0.0):
        return 0.0
    return (yc * yc / sc + ys * ys / ss) / (2.0 * variance)


def fap(peak, bins):
    return 1.0 - (1.0 - math.exp(-peak)) ** bins


def golden():
    values = values_of(TIMES)
    n = len(TIMES)
    mean = sum(values) / n
    variance = sum((v - mean) ** 2 for v in values) / (n - 1)
    y = [v - mean for v in values]
    freqs, baseline, m = derive_grid(TIMES)
    powers = [ls_power(f, TIMES, y, variance) for f in freqs]
    best = 0
    for i, p in enumerate(powers):
        if p > powers[best]:
            best = i
    total = sum(powers)
    half_band = 1.0 / baseline
    band = sum(p for f, p in zip(freqs, powers) if abs(f - freqs[best]) <= half_band)
    return freqs, powers, baseline, m, best, band / total, fap(powers[best], m)


def main():
    freqs, powers, baseline, m, best, frac, p = golden()
    print(f"baseline = {baseline!r}")
    print(f"grid_size = {m!r}")
    print(f"peak_index = {best}")
    print(f"peak_freq = {freqs[best]!r}")
    print(f"peak_power = {powers[best]!r}")
    print(f"power_fraction = {frac!r}")
    print(f"p_value = {p!r}")
    print("# pinned powers subset (every 25th bin + the first + the last):")
    idxs = list(range(0, m, 25))
    if m - 1 not in idxs:
        idxs.append(m - 1)
    for i in idxs:
        print(f"  ({i}, {powers[i]!r}),")

    # ── the third-implementation cross-check (scipy) ──────────────────
    # scipy.signal.lombscargle consumes ANGULAR frequencies (ω = 2πf) and
    # normalizes by 2/Σy² (uncentered, ddof=0) when normalize=True; the
    # contract normalizes by 2σ² with σ² ddof=1 about the mean. The exact
    # conversion for pre-centered input: P_contract = P_scipy · (N−1)/2
    # (Σy² = N·var₀, σ² = Σy²/(N−1) ⇒ N·var₀/(2σ²) = (N−1)/2). The shape
    # must then match to float noise — the verdict below is the DoD's
    # «эталонные значения, посчитанные вне проекта».
    try:
        import numpy as np
        from scipy.signal import lombscargle

        values = values_of(TIMES)
        mean = sum(values) / len(values)
        y = np.array([v - mean for v in values])
        t = np.array(TIMES)
        p_scipy = lombscargle(t, y, np.array(freqs) * 2.0 * np.pi, normalize=True)
        scale = (len(TIMES) - 1) / 2.0
        rel = np.abs(p_scipy * scale - np.array(powers)) / np.array(powers)
        print(f"scipy argmax = {int(np.argmax(p_scipy))} (ours {best})")
        print(f"scipy max relative deviation after the (N-1)/2 conversion = {rel.max():.3e}")
        verdict = "PASS" if rel.max() < 1e-9 and int(np.argmax(p_scipy)) == best else "FAIL"
        print("scipy cross-check:", verdict)
    except ImportError:
        print("scipy unavailable — the cross-check ran on the plain-Python only")


if __name__ == "__main__":
    main()
