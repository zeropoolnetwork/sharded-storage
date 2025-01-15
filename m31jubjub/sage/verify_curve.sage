from functools import reduce
import hashlib

p = 2^31-1
q = p^8

# x^2 = -1
# y^2 = x+2
# z^2 = y
#
# z^4 = y^2 = x+2
# (z^4-2)^2 = z^8 - 4z^4 + 4 = x^2 = -1
# z^8 - 4z^4 + 5 = 0 

Fp = GF(p)
R.<x> = Fp[]
Fp8.<x> = GF(q, modulus=x^8-4*x^4+5)


def to_shifed_poly(l, o):
    return sum([e * (x+o)^i for (i, e) in enumerate(l)])

def poly_to_tower_f2(poly):
    l = list(poly)
    l = l + [Fp(0)] * (2 - len(l))
    return (poly[0], poly[1])

def poly_to_tower_f4(poly):
    l = list(poly)
    l = l + [Fp(0)] * (4 - len(l))
    even = to_shifed_poly(l[0::2], 2)
    odd = to_shifed_poly(l[1::2], 2)
    return (poly_to_tower_f2(even), poly_to_tower_f2(odd))

def poly_to_tower_f8(poly):
    l = list(poly)
    l = l + [Fp(0)]*(8-len(l))
    even = to_shifed_poly(l[0::2], 0)
    odd = to_shifed_poly(l[1::2], 0)
    return (poly_to_tower_f4(even), poly_to_tower_f4(odd))


def poly_to_quadratic_complex(poly):
    l = list(poly)
    l = l + [Fp(0)]*(8-len(l))
    res = []
    for i in range(0,4):
        # z^4 = 2 + x
        res.append((l[i] + 2*l[i+4], l[i+4]))
    return res


cofactor = 8
twist_cofactor = 4

safety_bits = 121

A = x + 76823

E = EllipticCurve(Fp8, [0, A, 0, 1, 0])

## non-square value for creating twist
B = 4 + x
assert not B.is_square()

Etwist = EllipticCurve(Fp8, [0, B*A, 0, B, 0])

num_points = E.cardinality()
twist_num_points = 2*(q+1) - num_points

## find generators
hash_bytes = hashlib.sha256("m31jubjub".encode()).digest()
gen_x = sum([int.from_bytes(hash_bytes[i:i+4], "big") * x^(i//4) for i in range(0, len(hash_bytes), 4)])

G=None
G8=None
while True:
    while not (gen_x^3+A*gen_x^2+gen_x).is_square():
        gen_x += 1
    gen_y = (gen_x^3+A*gen_x^2+gen_x).sqrt()
    G = E(gen_x, gen_y)
    G8 = cofactor*G
    if G*(num_points/2)!=E(0) and G*8 != E(0):
        break
    gen_x += 1



## safecurve disc
trace = q+1 - num_points
K=trace^2 - 4*q
s = reduce(lambda x, y: x*(y[0]^(y[1]//2)), factor(K), 1)
D = K/(s^2)
if D % 4 != 1:
    D = 4*D

## safecurve twist disc
trace = q+1 - twist_num_points
K=trace^2 - 4*q
s = reduce(lambda x, y: x*(y[0]^(y[1]//2)), factor(K), 1)
Dtwist = K/(s^2)
if Dtwist % 4 != 1:
    Dtwist = 4*Dtwist



num_points_euler = euler_phi(num_points)
num_points_euler_factor = factor(num_points_euler)

embedding_degree = num_points_euler
for i in num_points_euler_factor:
    while pow(p, embedding_degree // i[0], num_points) == 1:
        embedding_degree = embedding_degree // i[0]

num_points_euler = euler_phi(twist_num_points)
num_points_euler_factor = factor(num_points_euler)

embedding_degree_twist = num_points_euler
for i in num_points_euler_factor:
    while pow(p, embedding_degree_twist // i[0], twist_num_points) == 1:
        embedding_degree_twist = embedding_degree_twist // i[0]




assert E.discriminant() != 0, "Discriminant is zero"
assert E.j_invariant() != 0, "j-invariant is zero"
assert E.j_invariant() != 1728, "j-invariant is 1728"

assert Etwist.discriminant() != 0, "Discriminant of twist is zero"
assert Etwist.j_invariant() != 0, "j-invariant of twist is zero"
assert Etwist.j_invariant() != 1728, "j-invariant of twist is 1728"


assert num_points % cofactor == 0, "Number of points is not divisible by cofactor"
assert twist_num_points % twist_cofactor == 0, "Twist number of points is not divisible by twist cofactor"
assert (num_points // cofactor).is_prime(), "Number of points is not cofactor * prime"
assert (twist_num_points // twist_cofactor).is_prime(), "Twist number of points is not twist cofactor * prime"

assert not (A^2 - 4).is_square(), "Non-trivial 2-torsion point exists"

assert (0.886*sqrt(num_points)).log(2) >= safety_bits, "Error: safecurve rho"
assert (0.886*sqrt(twist_num_points)).log(2) >= safety_bits, "Error: safecurve twist rho"

        
assert embedding_degree.log(2) >= 200, "Error: safecurve transfer"
assert embedding_degree_twist.log(2) >= 200, "Error: safecurve twist transfer"

assert abs(D).log(2) > 100, "Error: safecurve disc"
assert abs(Dtwist).log(2) > 100, "Error: safecurve twist disc"

a_ = A+2
d_ = A-2
d = d_/a_
a_sqrt = a_.sqrt()



A_tower = poly_to_tower_f8(A)
A_quadratic_complex = poly_to_quadratic_complex(A)

G_tower = (poly_to_tower_f8(G[0]), poly_to_tower_f8(G[1]))
G8_tower = (poly_to_tower_f8(G8[0]), poly_to_tower_f8(G8[1]))

G_quadratic_complex = (poly_to_quadratic_complex(G[0]), poly_to_quadratic_complex(G[1]))
G8_quadratic_complex = (poly_to_quadratic_complex(G8[0]), poly_to_quadratic_complex(G8[1]))

ed_g = (G[0]*a_sqrt/G[1], (G[0]-1)/(G[0]+1))
ed_g_tower = (poly_to_tower_f8(G[0]), poly_to_tower_f8(G[1]))
ed_g_quadratic_complex = (poly_to_quadratic_complex(ed_g[0]), poly_to_quadratic_complex(ed_g[1]))

ed_g8 = (G8[0]*a_sqrt/G8[1], (G8[0]-1)/(G8[0]+1))
ed_g8_tower = (poly_to_tower_f8(G8[0]), poly_to_tower_f8(G8[1]))
ed_g8_quadratic_complex = (poly_to_quadratic_complex(ed_g8[0]), poly_to_quadratic_complex(ed_g8[1]))

d_tower = poly_to_tower_f8(d)
d_quadratic_complex = poly_to_quadratic_complex(d)

print("All checks passed completely")
print(f"""

p={p}
F_p^8 irreducible polynomial: {Fp8.modulus()}

Montgomery form of curve y^2 = x^3 + A x^2 + x, where

A = {A} = {A_quadratic_complex}

num_points = {num_points}

subgroup_order = {num_points // cofactor}

G = {G} = {G_quadratic_complex}

G8 = {G8} = {G8_quadratic_complex}



Edwards form of curve x^2 + y^2 = 1 + d x^2 y^2, where

d = {d} ={d_quadratic_complex}

ED_G =  {ed_g} = {ed_g_quadratic_complex}

ED_G8 = {ed_g8} = {ed_g8_quadratic_complex}
""")