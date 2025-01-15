use p3_field::{AbstractField, Field}; 
use core::ops::{Add, Mul, Neg, Sub, AddAssign, MulAssign, SubAssign};
use core::cmp::Eq;
use ark_ff::{PrimeField, BigInteger};
use num_bigint::BigUint;

pub trait CurveParams: Sized + Clone + Copy {
    type Fq: Field;
    type Fs: PrimeField;
    const D: Self::Fq;
    const G: PointProjective<Self>;
    const G8: PointProjective<Self>;
}

#[derive(Debug, Clone, Copy)]
pub struct Point<P:CurveParams> {
    pub x: P::Fq,
    pub y: P::Fq
}

impl <P:CurveParams> PartialEq for Point<P> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl <P:CurveParams> Eq for Point<P> {}

impl <P:CurveParams> Point<P> {
    pub const fn new(x: P::Fq, y: P::Fq) -> Self {
        Point { x, y }
    }

    pub fn zero() -> Self {
        Point {
            x: P::Fq::zero(),
            y: P::Fq::one()
        }
    }

    pub fn subgroup_decompress(x: P::Fq) -> Option<Self> {
        let x2 = x.square();
        let y2 = (x2 - P::Fq::one()) / (P::D * x2 - P::Fq::one());

        let y = y2.sqrt()?;
        let res = Point { x, y };

        if res.is_in_subgroup() {
            Some(res)
        } else {
            let res = Point { x, y: -y };
            if res.is_in_subgroup() {
                Some(res)
            } else {
                None
            }
        }
    }

    pub fn is_on_curve(&self) -> bool {
        let x2 = self.x.square();
        let y2 = self.y.square();
        let lhs = x2 + y2;
        let rhs = P::Fq::one() + P::D * x2 * y2;
        lhs == rhs
    }

    // assuming is_on_curve is true
    pub fn is_in_subgroup(&self) -> bool {
        let p = PointProjective::from(*self);
        let q = p * P::Fs::MODULUS;
        q == PointProjective::zero()
    }
}

impl <P:CurveParams> From<PointProjective<P>> for Point<P> {
    fn from(p: PointProjective<P>) -> Self {
        let z_inv = p.z.inverse();
        Point {
            x: p.x * z_inv,
            y: p.y * z_inv
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PointProjective<P:CurveParams> {
    x: P::Fq,
    y: P::Fq,
    z: P::Fq
}

impl <P:CurveParams> PointProjective<P> {
    pub const fn new(x: P::Fq, y: P::Fq, z: P::Fq) -> Self {
        PointProjective { x, y, z }
    }

    pub fn zero() -> Self {
        PointProjective {
            x: P::Fq::zero(),
            y: P::Fq::one(),
            z: P::Fq::one()
        }
    }

    // https://www.hyperelliptic.org/EFD/g1p/auto-edwards-projective.html#add-2007-bl
    //  B = (X1+Y1)2
    //  C = X12
    //  D = Y12
    //  E = C+D
    //  H = (c*Z1)2
    //  J = E-2*H
    //  X3 = c*(B-E)*J
    //  Y3 = c*E*(C-D)
    //  Z3 = E*J
    
    pub fn double(&self) -> Self {
        let b = (self.x + self.y).square();
        let c = self.x.square();
        let d = self.y.square();
        let e = c + d;
        let h = self.z.square();
        let j = e - h.double();
        let x3 = (b - e) * j;
        let y3 = e * (c - d);
        let z3 = e * j;
        PointProjective {
            x: x3,
            y: y3,
            z: z3
        }
    }

    pub fn is_on_curve(&self) -> bool {
        let x2 = self.x.square();
        let y2 = self.y.square();
        let z2 = self.z.square();
        let z4 = z2.square();
        let lhs = (x2 + y2) * z2;
        let rhs = z4 + P::D * x2 * y2;
        lhs == rhs
    }
}

impl <P:CurveParams> From<Point<P>> for PointProjective<P> {
    fn from(p: Point<P>) -> Self {
        PointProjective {
            x: p.x,
            y: p.y,
            z: P::Fq::one()
        }
    }
}

impl <P:CurveParams> PartialEq for PointProjective<P> {
    fn eq(&self, other: &Self) -> bool {
        let a = self.x * other.z;
        let b = other.x * self.z;
        let c = self.y * other.z;
        let d = other.y * self.z;
        a == b && c == d
    }
}   

impl <P:CurveParams> Eq for PointProjective<P> {}

impl <P:CurveParams> Add<PointProjective<P>> for PointProjective<P> {
    type Output = PointProjective<P>;

    // From https://www.hyperelliptic.org/EFD/g1p/auto-edwards-projective.html#add-2007-bl
    // A = Z1*Z2
    // B = A2
    // C = X1*X2
    // D = Y1*Y2
    // E = d*C*D
    // F = B-E
    // G = B+E
    // X3 = A*F*((X1+Y1)*(X2+Y2)-C-D)
    // Y3 = A*G*(D-C)
    // Z3 = c*F*G

    fn add(self, other: PointProjective<P>) -> PointProjective<P> {
        let a = self.z * other.z;
        let b = a.square();
        let c = self.x * other.x;
        let d = self.y * other.y;
        let e = P::D * c * d;
        let f = b - e;
        let g = b + e;
        let x3 = a * f * ((self.x + self.y) * (other.x + other.y) - c - d);
        let y3 = a * g * (d - c);
        let z3 = f * g;

        PointProjective {
            x: x3,
            y: y3,
            z: z3
        }
    }
}

impl <P:CurveParams> Neg for PointProjective<P> {
    type Output = PointProjective<P>;

    fn neg(self) -> PointProjective<P> {
        PointProjective {
            x: -self.x,
            y: self.y,
            z: self.z
        }
    }
}

impl <P:CurveParams> Sub<PointProjective<P>> for PointProjective<P> {
    type Output = PointProjective<P>;

    fn sub(self, other: PointProjective<P>) -> PointProjective<P> {
        self + (-other)
    }
}

impl <P:CurveParams> AddAssign<PointProjective<P>> for PointProjective<P> {
    fn add_assign(&mut self, other: PointProjective<P>) {
        *self = *self + other;
    }
}

impl <P:CurveParams> SubAssign<PointProjective<P>> for PointProjective<P> {
    fn sub_assign(&mut self, other: PointProjective<P>) {
        *self = *self - other;
    }
}

impl <P:CurveParams, IntoBigUint:Into<BigUint>> Mul<IntoBigUint> for PointProjective<P> {
    type Output = PointProjective<P>;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, scalar: IntoBigUint) -> PointProjective<P> {
        let scalar: BigUint = scalar.into();
        let mut res = PointProjective {
            x: P::Fq::zero(),
            y: P::Fq::one(),
            z: P::Fq::one()
        };

        for i in (0.. scalar.bits()).rev() {
            res = res.double();
            if scalar.bit(i) {
                
                res += self;
            }
        }

        res
    }
}

impl <P:CurveParams, BigInt:BigInteger> MulAssign<BigInt> for PointProjective<P> {
    fn mul_assign(&mut self, scalar: BigInt) {
        *self = *self * scalar;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::m31::{Fq, fq_new_from_raw, M31JubJubParams};
    use num_bigint::BigUint;


    #[test]
    fn test_point_addition() {
        let p1 = PointProjective::<M31JubJubParams>::new(
            fq_new_from_raw([656773052, 1049042311, 261343438, 978776757, 1117968940, 1193107093, 1754133089, 1611118327]),
            fq_new_from_raw([1487266101, 1175747075, 505027441, 1763816805, 740435462, 1577690028, 935897188, 866866833]),
            Fq::one()
        );
        let p2 = PointProjective::<M31JubJubParams>::new(
            fq_new_from_raw([1079462407, 1454729686, 589577511, 1412220565, 602427144, 100971497, 1401486583, 1639190599]),
            fq_new_from_raw([537019307, 1205844760, 2047831322, 857711787, 75482716, 680931946, 1651671436, 783040619]),
            Fq::one()
        );
        let p3 = p1 + p2;
        assert_eq!(p3, PointProjective::<M31JubJubParams>::new(
            fq_new_from_raw([639234870, 119585210, 2012456890, 718424677, 757216708, 1678867623, 821452381, 989546821]),
            fq_new_from_raw([1493894487, 745890410, 1521415160, 342400207, 649101696, 1199961840, 2040117174, 1685998621]),
            Fq::one()
        ));
    }

    #[test]
    fn test_point_subtraction() {
        let p1 = PointProjective::<M31JubJubParams>::new(
            fq_new_from_raw([420101081, 576412224, 1167411183, 161364634, 1139146199, 125994107, 1187546699, 834208552]),
            fq_new_from_raw([1588768746, 1515797309, 1952320959, 852870876, 1768507089, 859951467, 1604259815, 730189172]),
            Fq::one()
        );
        let p2 = PointProjective::<M31JubJubParams>::new(
            fq_new_from_raw([1557087308, 1235848012, 1053862800, 1331307504, 1373382043, 1687829064, 120274533, 109936380]),
            fq_new_from_raw([1057487233, 1609034853, 1644877774, 740898323, 1278450500, 409559954, 687197396, 2138368490]),
            Fq::one()
        );
        let p3 = p1 - p2;
        assert_eq!(p3, PointProjective::<M31JubJubParams>::new(
            fq_new_from_raw([892236054, 546274057, 700175836, 1883669643, 2002893632, 445924307, 242026564, 1508998525]),
            fq_new_from_raw([83638949, 1006846367, 2113339843, 611690257, 690662445, 1777439948, 890512936, 1545469441]),
            Fq::one()
        ));
    }

    #[test]
    fn test_point_negation() {
        let p = PointProjective::<M31JubJubParams>::new(
            fq_new_from_raw([957017432, 1234079227, 1639807341, 455567724, 2129752570, 2042243010, 58053351, 1891312594]),
            fq_new_from_raw([1997287292, 529661678, 1147434582, 1138801199, 235918777, 1051658595, 280240491, 1534928828]),
            Fq::one()
        );
        let p_neg = -p;
        assert_eq!(p_neg, PointProjective::<M31JubJubParams>::new(
            fq_new_from_raw([1190466215, 913404420, 507676306, 1691915923, 17731077, 105240637, 2089430296, 256171053]),
            fq_new_from_raw([1997287292, 529661678, 1147434582, 1138801199, 235918777, 1051658595, 280240491, 1534928828]),
            Fq::one()
        ));
    }

    #[test]
    fn test_point_multiplication() {
        let p = PointProjective::<M31JubJubParams>::new(
            fq_new_from_raw([1474818382, 789665590, 1131336187, 777975669, 1472625174, 1911349612, 539431508, 1295623315]),
            fq_new_from_raw([911569315, 945670610, 1025358386, 999097567, 551320493, 565211807, 1419321184, 1568833412]),
            Fq::one()
        );
        let scalar = BigUint::parse_bytes(b"3298207776612928512897698571044484781110597956891190871319344352830623685", 10).unwrap();
        let p2 = p * scalar;
        assert_eq!(p2, PointProjective::<M31JubJubParams>::new(
            fq_new_from_raw([13031194, 1559842476, 133367014, 598331427, 1094458529, 116942584, 1119817973, 1646113635]),
            fq_new_from_raw([1637911753, 124009272, 1233823825, 1377679743, 428001688, 745005718, 620789987, 1361258141]),
            Fq::one()
        ));
    }

    #[test]
    fn test_is_on_curve_positive() {
        let p = Point::<M31JubJubParams>::new(
            fq_new_from_raw([823947976, 1923116504, 1620555214, 1284834718, 752429251, 1214229998, 1144720752, 210310495]),
            fq_new_from_raw([1353430588, 961741731, 1466699004, 980344758, 1078492372, 1276883424, 54524486, 309667334])
        );
        assert!(p.is_on_curve());
    }

    #[test]
    fn test_is_on_curve_negative() {
        let p = Point::<M31JubJubParams>::new(
            fq_new_from_raw([823947976, 1923116504, 1620555214, 1284834718, 752429251, 1214229998, 1144720752, 210310495]),
            fq_new_from_raw([1353430588, 961741731, 1466699004, 980344758, 1078492372, 1276883424, 54524486, 309667334]) + Fq::one()
        );
        assert!(!p.is_on_curve());
    }

    #[test]
    fn test_is_in_subgroup_positive() {
        let p = Point::<M31JubJubParams>::new(
            fq_new_from_raw([1808584124, 136304426, 1090368464, 2026805467, 1256499828, 633931140, 2028004058, 165646578]),
            fq_new_from_raw([824519854, 1504142516, 417214436, 1179166135, 375981804, 148599853, 54408067, 434472909])
        );
        assert!(p.is_in_subgroup());
    }

    #[test]
    fn test_is_in_subgroup_negative() {
        let p = Point::<M31JubJubParams>::new(
            fq_new_from_raw([1808584124, 136304426, 1090368464, 2026805467, 1256499828, 633931140, 2028004058, 165646578]),
            fq_new_from_raw([824519854, 1504142516, 417214436, 1179166135, 375981804, 148599853, 54408067, 434472909])
        );
        let p: Point<_> = (PointProjective::from(p) + M31JubJubParams::G).into();
        assert!(!p.is_in_subgroup());
    }

    #[test]
    fn test_subgroup_decompress() {
        let x = fq_new_from_raw([1808584124, 136304426, 1090368464, 2026805467, 1256499828, 633931140, 2028004058, 165646578]);
        let y = fq_new_from_raw([824519854, 1504142516, 417214436, 1179166135, 375981804, 148599853, 54408067, 434472909]);
        let decompressed_point = Point::<M31JubJubParams>::subgroup_decompress(x);
        assert_eq!(decompressed_point.unwrap(), Point::<M31JubJubParams>::new(x, y));
    }



    #[test]
    fn test_generators_relation() {
        let g = M31JubJubParams::G;
        let g8 = M31JubJubParams::G8;
        let g8_alternate = g * BigUint::from(8u8);
        assert_eq!(g8, g8_alternate);
    }
}

