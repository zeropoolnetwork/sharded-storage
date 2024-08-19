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
- $\mathbb{F}_{p^2} = \mathbb{F}_p[x] / (x^2 + 1)$
- $\mathbb{F}_{p^4} = \mathbb{F}_{p^2}[y] / (y^2 - (x+2))$
- $\mathbb{F}_{p^8} = \mathbb{F}_{p^4}[z] / (z^2 - y)$

This tower of extensions allows for efficient field arithmetic and is crucial for the implementation of the m31jubjub curve.

### 📈 Montgomery Curve
The curve is defined in Montgomery form as:
$y^2 = x^3 + Ax^2 + x$
where $A = x + 76823 = (((76823, 0), (0, 0)), ((1, 0), (0, 0)))$

#### 🔢 Curve Properties
- Number of points: 452312846898269724422641179697543667437631587593405403658152560327606217112
- Subgroup order: 56539105862283715552830147462192958429703948449175675457269070040950777139

#### 🎯 Generator Points
- G = $(907814216x^7 + 1363579468x^6 + 750444619x^5 + 1179007401x^4 + 929387331x^3 + 1952555432x^2 + 948184434x + 653256504 :$ 
     $275602113x^7 + 1702114697x^6 + 446309175x^5 + 343110055x^4 + 1648926999x^3 + 986304373x^2 + 274761817x + 1519537697 : 1)$

- G8 = $(120323281x^7 + 984300192x^6 + 333124324x^5 + 193728290x^4 + 971865944x^3 + 184940195x^2 + 985511338x + 1124927172 :$ 
      $794190723x^7 + 402581624x^6 + 1264254179x^5 + 256913164x^4 + 202670437x^3 + 911142522x^2 + 2127907999x + 251890364 : 1)$

### 📉 Edwards Curve
The equivalent Edwards curve is defined as:
$x^2 + y^2 = 1 + dx^2y^2$
where $d = 1236191318x^7 + 2066283225x^6 + 1929909262x^5 + 1286903024x^4 + 1025007455x^3 + 2027605115x^2 + 1258342564x + 1103857700$

#### 🎯 Edwards Generator Points
- ED_G = $(455926870x^7 + 1084608143x^6 + 1907750992x^5 + 625092471x^4 + 33256378x^3 + 1030900550x^2 + 1333002994x + 627452245,$
         $412915271x^7 + 1935691581x^6 + 1692807047x^5 + 892421824x^4 + 635330314x^3 + 583922426x^2 + 1052874821x + 363807993)$

- ED_G8 = $(464152701x^7 + 365047665x^6 + 599270435x^5 + 662698927x^4 + 1784010339x^3 + 1415137703x^2 + 362910707x + 1690521432,$
          $1973006813x^7 + 1293690843x^6 + 1429284693x^5 + 1089547304x^4 + 336185120x^3 + 709283412x^2 + 861786203x + 1469941854)$

## 📚 References

This project is based on the following works:
- [jubjub](https://github.com/daira/jubjub): A high-security elliptic curve for zero-knowledge proof systems
- [SSS22](https://eprint.iacr.org/2022/277): "Faster Computation in the Algebraic Group Model" by Robin Salen, Vijaykumar Singh, Vladimir Soukharev