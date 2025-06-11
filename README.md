# Improved gls254 (crrl)
This is an improved version of the gls254 module from the [crrl](https://github.com/pornin/crrl) library.

The following files have been updated from crrl version 0.9.0:
- src/gls254.rs
- benches/gls254.rs

## Runs
The command below can be used to run this. For more details, please refer to the [crrl](https://github.com/pornin/crrl) library and [1].

```
cargo bench --no-default-features -F gls254,gls254bench
```

## Benchmarks

Our ECSM, called AD-2DT, achieves 29104 cycles on an Intel x86 CPU, which improves the best previous records by 8.1%.

| Variant  | This work | | Variant | Previous work [1]|
|----------|-----------|-|---------|------------------|
| AD-1DT-3 | 31499     | | 1DT-3   | 35102 (35383)    |
| AD-1DT-4 | 29578     | | 1DT-4   | **31657** (31615)    |
| AD-1DT-5 | 31079     | | 1DT-5   | 32670 (31785)    |
| AD-2DT-2 | **29104**     | | 2DT-2   | 32043 (32583)    |
| AD-2DT-3 | 30889     | | 2DT-3   | 32966 (32275)    |

Table 1: Performance of raw ECDH on Intel x86 Broadwell-class CPUs using pclmulqdq. Performance is measured in clock cycles. Values in parentheses show results reported in existing papers [1], measured on Intel x86 Skylake-class CPUs.

## References

[1] Thomas Pornin. Faster complete formulas for the GLS254 binary curve. IACR Cryptol. ePrint Arch., page 1688, 2023. https://eprint.iacr.org/2023/1688.