# m31jubjub curve

> ⚠️ **WARNING**: This code has not been audited and is not intended for production use. It is an experimental implementation and should be used for educational or research purposes only.

This project implements the m31jubjub elliptic curve, which is defined over the field $\mathbb{F}_{p^8}$, where $p$ is the Mersenne31 prime $2^{31}-1$. It is based on the principles from [jubjub](https://github.com/daira/jubjub) and [SSS22](https://eprint.iacr.org/2022/277).

## 🔢 Field and Curve Parameters

### 📊 Field Definition
- Prime: $p = 2^{31} - 1 = 2147483647$ (Mersenne31 prime)
- Field: $\mathbb{F}_{p^8}$

### 🔠 Irreducible Polynomial and Field Extensions
The irreducible polynomial defining $\mathbb{F}_{p^8}$ is:
$x^8 + 2147483643x^4 + 5$

This polynomial is derived from the following relationships, which also define the intermediate field extensions:

$x^2 = -1$ (defines $\mathbb{F}_{p^2}$)

$y^2 = x+2$ (defines $\mathbb{F}_{p^4}$ over $\mathbb{F}_{p^2}$)

$z^2 = y$ (defines $\mathbb{F}_{p^8}$ over $\mathbb{F}_{p^4}$)

From these, we can derive:

$z^4 = y^2 = x+2$

$(z^4-2)^2 = z^8 - 4z^4 + 4 = x^2 = -1$

$z^8 - 4z^4 + 5 = 0$

These relationships lead to the construction of the irreducible polynomial $x^8 -4 x^4 + 5$.

The field extensions are constructed as follows:

- $\mathbb{F}_{p^2} = \mathbb{F} _p[x] / (x^2 + 1)$
- $\mathbb{F}_{p^4} = \mathbb{F} _{p^2}[y] / (y^2 - (x+2))$
- $\mathbb{F}_{p^8} = \mathbb{F} _{p^4}[z] / (z^2 - y)$

This tower of extensions allows for efficient field arithmetic and is crucial for the implementation of the m31jubjub curve.

### 📈 Montgomery Curve
The curve is defined in Montgomery form as:
$y^2 = x^3 + Ax^2 + x$
where $A = (((76823, 0), (0, 0)), ((1, 0), (0, 0)))$

#### 🔢 Curve Properties
- Number of points: 452312846898269724422641179697543667437631587593405403658152560327606217112
- Subgroup order: 56539105862283715552830147462192958429703948449175675457269070040950777139

#### 🎯 Generator Points
- G = $([(1512383752, 193728290), (1651759986, 333124324), (6056932, 984300192), (1212512506, 120323281)],$
      $[(58274160, 343110055), (1167380167, 446309175), (95566473, 1702114697), (52647578, 275602113)])$

- G8 = $([(1512383752, 193728290), (1651759986, 333124324), (6056932, 984300192), (1212512506, 120323281)],$
      $[(765716692, 256913164), (361449063, 1264254179), (1716305770, 402581624), (1791051883, 794190723)])$

### 📉 Edwards Curve
The equivalent Edwards curve is defined as:
$x^2 + y^2 = 1 + dx^2y^2$
where $d = [(1530180101, 1286903024), (823193794, 1929909262), (1865204271, 2066283225), (1349906444, 1236191318)]$

#### 🎯 Edwards Generator Points
- ED_G = $([(1877637187, 625092471), (853537684, 1907750992), (1052633189, 1084608143), (945110118, 455926870)],$
      $[(1167994, 892421824), (143521621, 1692807047), (160338294, 1935691581), (1461160856, 412915271)])$

- ED_G8 = $([(1279048008, 1484784720), (586032070, 1548213212), (2250614, 1782435982), (1582651553, 1683330946)],$
$[(1501552815, 1089547304), (1572871942, 1429284693), (1149181451, 1293690843), (2134715099, 1973006813)])$

## 📚 References

This project is based on the following works:
- [jubjub](https://github.com/daira/jubjub): A high-security elliptic curve for zero-knowledge proof systems
- [SSS22](https://eprint.iacr.org/2022/277): "Faster Computation in the Algebraic Group Model" by Robin Salen, Vijaykumar Singh, Vladimir Soukharev
