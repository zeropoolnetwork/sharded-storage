# m31jubjub curve

> ⚠️ **WARNING**: This code has not been audited and is not intended for production use. It is an experimental implementation and should be used for educational or research purposes only.

This project implements the m31jubjub elliptic curve, which is defined over the field $\mathbb{F}_{p^8}$, where $p$ is the Mersenne31 prime $2^{31}-1$. It is based on the principles from [jubjub](https://github.com/daira/jubjub) and [SSS22](https://eprint.iacr.org/2022/277).

## 🔢 Field and Curve Parameters

### 📊 Field Definition
- Prime: $p = 2^{31} - 1 = 2147483647$ (Mersenne31 prime)
- Field: $\mathbb{F}_{p^8}$

### 🔠 Irreducible Polynomial
The irreducible polynomial defining $\mathbb{F}_{p^8}$ is:
$x^8 + x^7 + 3x^6 + 11x^5 + 44x^4 + 2147483594x^3 + 153x^2 + 2147483487x + 59$

### 📈 Montgomery Curve
The curve is defined in Montgomery form as:
$y^2 = x^3 + Ax^2 + x$
where $A = z + 26867$

#### 🔢 Curve Properties
- Number of points: 452312846898269724422641179697543667467814320070513988227972083247142673448
- Subgroup order: 56539105862283715552830147462192958433476790008814248528496510405892834181

#### 🎯 Generator Points
- G = $(1176524345z^7 + 1614722485z^6 + 1771761530z^5 + 1037047461z^4 + 1902949416z^3 + 1681296208z^2 + 1439155598z + 701739277 :$ 
     $513072117z^7 + 435962782z^6 + 52306192z^5 + 980542388z^4 + 628795505z^3 + 2059070988z^2 + 207361672z + 1042491461 : 1)$

- G8 = $(1540365301z^7 + 948377362z^6 + 1207546392z^5 + 1517929562z^4 + 1165344054z^3 + 1658385247z^2 + 12528530z + 1643334121 :$ 
      $1116715088z^7 + 500092626z^6 + 1448528837z^5 + 227008444z^4 + 1723897891z^3 + 532578650z^2 + 1645207566z + 1848300397 : 1)$

### 📉 Edwards Curve
The equivalent Edwards curve is defined as:
$x^2 + y^2 = 1 + dx^2y^2$
where $d = 2002400458z^7 + 412302747z^6 + 270376163z^5 + 751622722z^4 + 1791573242z^3 + 1458949930z^2 + 1065415368z + 1049965489$

#### 🎯 Edwards Generator Points
- ED_G = $(1868233130z^7 + 1845635381z^6 + 996200517z^5 + 1671418165z^4 + 1352036533z^3 + 1102934054z^2 + 968933842z + 1606261697,$
         $1203533863z^7 + 1358649025z^6 + 425771115z^5 + 1464258342z^4 + 596504205z^3 + 947452962z^2 + 855861250z + 1664890207)$

- ED_G8 = $(1953503707z^7 + 281564145z^6 + 950234997z^5 + 1534170886z^4 + 1296704577z^3 + 1760437366z^2 + 1057881548z + 898105883,$
          $2136715292z^7 + 1979089404z^6 + 320082435z^5 + 869134159z^4 + 527661356z^3 + 2073224530z^2 + 285880509z + 42694086)$


## 📚 References

This project is based on the following works:
- [jubjub](https://github.com/daira/jubjub): A high-security elliptic curve for zero-knowledge proof systems
- [SSS22](https://eprint.iacr.org/2022/277): "Faster Computation in the Algebraic Group Model" by Wahby, Steffen, and Straka
