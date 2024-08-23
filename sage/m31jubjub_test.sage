# Constants
p = 2^31 - 1
q = p^8

Fp = GF(p)
R.<x> = Fp[]
Fp8.<x> = GF(q, modulus=x^8-4*x^4+5)

subroup_order = 56539105862283715552830147462192958429703948449175675457269070040950777139
num_points = 452312846898269724422641179697543667437631587593405403658152560327606217112
cofactor = 8

A = x + 76823

E = EllipticCurve(Fp8, [0, A, 0, 1, 0])

G=(907814216*x^7 + 1363579468*x^6 + 750444619*x^5 + 1179007401*x^4 + 929387331*x^3 + 1952555432*x^2 + 948184434*x + 653256504, 275602113*x^7 + 1702114697*x^6 + 446309175*x^5 + 343110055*x^4 + 1648926999*x^3 + 986304373*x^2 + 274761817*x + 1519537697)
G8=(120323281*x^7 + 984300192*x^6 + 333124324*x^5 + 193728290*x^4 + 971865944*x^3 + 184940195*x^2 + 985511338*x + 1124927172, 794190723*x^7 + 402581624*x^6 + 1264254179*x^5 + 256913164*x^4 + 202670437*x^3 + 911142522*x^2 + 2127907999*x + 251890364)

G=E(*G)
G8=E(*G8)

assert E.order() == subroup_order * cofactor
assert G*cofactor == G8
assert G8*subroup_order == E(0)

def poly_to_quadratic_complex(poly):
    l = list(poly)
    l = l + [Fp(0)]*(8-len(l))
    res = []
    for i in range(0,4):
        # z^4 = 2 + x
        res.append(l[i] + 2*l[i+4])
        res.append(l[i+4])
    return res

a_ = A+2
d_ = A-2
d = d_/a_

a_sqrt = a_.sqrt()

def to_edwards_quadratic_complex(p):
    p = (Fp8(0), Fp8(1)) if p == E(0) else (p[0]*a_sqrt/p[1], (p[0]-1)/(p[0]+1))
    return (poly_to_quadratic_complex(p[0]), poly_to_quadratic_complex(p[1]))

# Generate Fs elements
Fs = GF(subroup_order)
random_scalar = Fs.random_element()
random_scalar8 = Fs.random_element()

# Generate unique random points for each test
random_scalar_add1 = Fs.random_element()
random_scalar_add2 = Fs.random_element()
random_scalar_sub1 = Fs.random_element()
random_scalar_sub2 = Fs.random_element()
random_scalar_neg = Fs.random_element()
random_scalar_mul = Fs.random_element()

# Compute points in Sage's representation
point_add1 = G * random_scalar_add1
point_add2 = G * random_scalar_add2
point_sub1 = G * random_scalar_sub1
point_sub2 = G * random_scalar_sub2
point_neg = G * random_scalar_neg
point_mul = G * random_scalar_mul

# Compute results in Sage's representation
result_add = point_add1 + point_add2
result_sub = point_sub1 - point_sub2
result_neg = -point_neg
result_mul = point_mul * random_scalar

random_point = to_edwards_quadratic_complex(G * random_scalar)
random_point8 = to_edwards_quadratic_complex(G8 * random_scalar8)

# Convert points to Edwards representation
random_point_add1 = to_edwards_quadratic_complex(point_add1)
random_point_add2 = to_edwards_quadratic_complex(point_add2)
random_point_sub1 = to_edwards_quadratic_complex(point_sub1)
random_point_sub2 = to_edwards_quadratic_complex(point_sub2)
random_point_neg = to_edwards_quadratic_complex(point_neg)
random_point_mul = to_edwards_quadratic_complex(point_mul)

# Convert results to Edwards representation
result_add = to_edwards_quadratic_complex(result_add)
result_sub = to_edwards_quadratic_complex(result_sub)
result_neg = to_edwards_quadratic_complex(result_neg)
result_mul = to_edwards_quadratic_complex(result_mul)

ed_g = to_edwards_quadratic_complex(G)
ed_g8 = to_edwards_quadratic_complex(G8)

# Generate Rust test module
rust_test_module = f"""
#[cfg(test)]
mod tests {{
    use super::*;
    use crate::m31::{{Fq, Fs, fq_new_from_raw}};
    use num_bigint::BigUint;

    #[derive(Clone, Copy, Debug)]
    struct TestParams;

    impl Params for TestParams {{
        type Fq = Fq;
        type Fs = Fs;
        const D: Self::Fq = fq_new_from_raw({poly_to_quadratic_complex(d)});
        const G: PointProjective<Self> = PointProjective::new(
            fq_new_from_raw({ed_g[0]}),
            fq_new_from_raw({ed_g[1]}),
            fq_new_from_raw([1, 0, 0, 0, 0, 0, 0, 0])
        );
        const G8: PointProjective<Self> = PointProjective::new(
            fq_new_from_raw({ed_g8[0]}),
            fq_new_from_raw({ed_g8[1]}),
            fq_new_from_raw([1, 0, 0, 0, 0, 0, 0, 0])
        );
    }}

    #[test]
    fn test_point_addition() {{
        let p1 = PointProjective::<TestParams>::new(
            fq_new_from_raw({random_point_add1[0]}),
            fq_new_from_raw({random_point_add1[1]}),
            Fq::one()
        );
        let p2 = PointProjective::<TestParams>::new(
            fq_new_from_raw({random_point_add2[0]}),
            fq_new_from_raw({random_point_add2[1]}),
            Fq::one()
        );
        let p3 = p1 + p2;
        assert_eq!(p3, PointProjective::<TestParams>::new(
            fq_new_from_raw({result_add[0]}),
            fq_new_from_raw({result_add[1]}),
            Fq::one()
        ));
    }}

    #[test]
    fn test_point_subtraction() {{
        let p1 = PointProjective::<TestParams>::new(
            fq_new_from_raw({random_point_sub1[0]}),
            fq_new_from_raw({random_point_sub1[1]}),
            Fq::one()
        );
        let p2 = PointProjective::<TestParams>::new(
            fq_new_from_raw({random_point_sub2[0]}),
            fq_new_from_raw({random_point_sub2[1]}),
            Fq::one()
        );
        let p3 = p1 - p2;
        assert_eq!(p3, PointProjective::<TestParams>::new(
            fq_new_from_raw({result_sub[0]}),
            fq_new_from_raw({result_sub[1]}),
            Fq::one()
        ));
    }}

    #[test]
    fn test_point_negation() {{
        let p = PointProjective::<TestParams>::new(
            fq_new_from_raw({random_point_neg[0]}),
            fq_new_from_raw({random_point_neg[1]}),
            Fq::one()
        );
        let p_neg = -p;
        assert_eq!(p_neg, PointProjective::<TestParams>::new(
            fq_new_from_raw({result_neg[0]}),
            fq_new_from_raw({result_neg[1]}),
            Fq::one()
        ));
    }}

    #[test]
    fn test_point_multiplication() {{
        let p = PointProjective::<TestParams>::new(
            fq_new_from_raw({random_point_mul[0]}),
            fq_new_from_raw({random_point_mul[1]}),
            Fq::one()
        );
        let scalar = BigUint::parse_bytes(b"{int(random_scalar)}", 10).unwrap();
        let p2 = p * scalar;
        assert_eq!(p2, PointProjective::<TestParams>::new(
            fq_new_from_raw({result_mul[0]}),
            fq_new_from_raw({result_mul[1]}),
            Fq::one()
        ));
    }}

    #[test]
    fn test_is_on_curve_positive() {{
        let p = Point::<TestParams>::new(
            fq_new_from_raw({random_point[0]}),
            fq_new_from_raw({random_point[1]})
        );
        assert!(p.is_on_curve());
    }}

    #[test]
    fn test_is_on_curve_negative() {{
        let p = Point::<TestParams>::new(
            fq_new_from_raw({random_point[0]}),
            fq_new_from_raw({random_point[1]}) + Fq::one()
        );
        assert!(!p.is_on_curve());
    }}

    #[test]
    fn test_is_in_subgroup_positive() {{
        let p = Point::<TestParams>::new(
            fq_new_from_raw({random_point8[0]}),
            fq_new_from_raw({random_point8[1]})
        );
        assert!(p.is_in_subgroup());
    }}

    #[test]
    fn test_is_in_subgroup_negative() {{
        let p = Point::<TestParams>::new(
            fq_new_from_raw({random_point8[0]}),
            fq_new_from_raw({random_point8[1]})
        );
        let p: Point<_> = (PointProjective::from(p) + TestParams::G).into();
        assert!(!p.is_in_subgroup());
    }}

    #[test]
    fn test_subgroup_decompress() {{
        let x = fq_new_from_raw({random_point8[0]});
        let y = fq_new_from_raw({random_point8[1]});
        let decompressed_point = Point::<TestParams>::subgroup_decompress(x);
        assert_eq!(decompressed_point.unwrap(), Point::<TestParams>::new(x, y));
    }}
}}
"""

print(rust_test_module)