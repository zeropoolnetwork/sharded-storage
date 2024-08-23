
use p3_field::{AbstractField, Field}; 
use core::ops::{Add, Mul, Neg, Sub, AddAssign, MulAssign, SubAssign};
use core::cmp::Eq;
use ark_ff::{PrimeField, BigInteger};
use num_bigint::BigUint;



pub trait Params: Sized + Clone + Copy {
    type Fq: Field;
    type Fs: PrimeField;
    const D: Self::Fq;
    const G: PointProjective<Self>;
    const G8: PointProjective<Self>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point<P:Params> {
    pub x: P::Fq,
    pub y: P::Fq
}

    
impl <P:Params> Point<P> {
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
            Some(Point { x, y: -y })
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


impl <P:Params> From<PointProjective<P>> for Point<P> {
    fn from(p: PointProjective<P>) -> Self {
        let z_inv = p.z.inverse();
        Point {
            x: p.x * z_inv,
            y: p.y * z_inv
        }
    }
}



#[derive(Debug, Clone, Copy)]
pub struct PointProjective<P:Params> {
    pub x: P::Fq,
    pub y: P::Fq,
    pub z: P::Fq
}

impl <P:Params> PointProjective<P> {
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
}


impl <P:Params> From<Point<P>> for PointProjective<P> {
    fn from(p: Point<P>) -> Self {
        PointProjective {
            x: p.x,
            y: p.y,
            z: P::Fq::one()
        }
    }
}

impl <P:Params> PartialEq for PointProjective<P> {
    fn eq(&self, other: &Self) -> bool {
        let a = self.x * other.z;
        let b = other.x * self.z;
        let c = self.y * other.z;
        let d = other.y * self.z;
        a == b && c == d
    }
}   

impl <P:Params> Eq for PointProjective<P> {}

impl <P:Params> Add<PointProjective<P>> for PointProjective<P> {
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

impl <P:Params> Neg for PointProjective<P> {
    type Output = PointProjective<P>;

    fn neg(self) -> PointProjective<P> {
        PointProjective {
            x: -self.x,
            y: self.y,
            z: self.z
        }
    }
}

impl <P:Params> Sub<PointProjective<P>> for PointProjective<P> {
    type Output = PointProjective<P>;

    fn sub(self, other: PointProjective<P>) -> PointProjective<P> {
        self + (-other)
    }
}

impl <P:Params> AddAssign<PointProjective<P>> for PointProjective<P> {
    fn add_assign(&mut self, other: PointProjective<P>) {
        *self = *self + other;
    }
}

impl <P:Params> SubAssign<PointProjective<P>> for PointProjective<P> {
    fn sub_assign(&mut self, other: PointProjective<P>) {
        *self = *self - other;
    }
}

impl <P:Params, IntoBigUint:Into<BigUint>> Mul<IntoBigUint> for PointProjective<P> {
    type Output = PointProjective<P>;

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
                res = res + self;
            }
        }

        res
    }
}

impl <P:Params, BigInt:BigInteger> MulAssign<BigInt> for PointProjective<P> {
    fn mul_assign(&mut self, scalar: BigInt) {
        *self = *self * scalar;
    }
}


