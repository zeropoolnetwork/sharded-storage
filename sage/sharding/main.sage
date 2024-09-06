from sage.misc.cachefunc import cached_function
import numpy as np


p = 2^31-1
Fp = GF(p)

R.<x> = Fp[]


Fp2.<x_2> = GF(p^2, modulus=x^2+1)
Fp3.<x_3> = GF(p^3, modulus=x^3-5)


circle_gen_log_order = 31
circle_gen_order = 2^circle_gen_log_order
circle_gen = Fp2.multiplicative_generator()^((p^2 - 1) / circle_gen_order)
assert circle_gen.multiplicative_order() == circle_gen_order
assert circle_gen.norm() == 1


@cached_function
def circle_subgroup_gen(log_order):
    return circle_gen^(2^(circle_gen_log_order - log_order))

@cached_function
def evaluation_domain(log_order):
    offset = circle_subgroup_gen(log_order+2)
    g = circle_subgroup_gen(log_order)
    return [(offset*g^i)[0] for i in range(2^log_order)]



def list_prod_sum(a, b):
    assert len(a) == len(b)
    return sum([a[i]*b[i] for i in range(len(a))])

def list_eq(a, b):
    return len(a)==len(b) and all([a[i]==b[i] for i in range(len(a))])


def is_power_of_2(x):
    return x & (x-1) == 0


def log2_strict(x):
    assert is_power_of_2(x)
    return x.bit_length()-1


def left_half(x):
    return x[:len(x)//2]


@cached_function
def reverse_bits(n, bits):
    result = 0
    for i in range(bits):
        result = (result << 1) | (n & 1)
        n >>= 1
    return result


@cached_function
def symmetric_cfft_permute_index(index, log_n):
    return reverse_bits(index, log_n)



def symmetric_cfft_permute(xs):
    log_n = log2_strict(len(xs))
    res = []
    for i in range(len(xs)):
        res.append(xs[symmetric_cfft_permute_index(i, log_n)])
    return res



"""
def twiddles(log_n):
    if log_n==0:
        return []
    res = []
    res.append(left_half(evaluation_domain(log_n)))
    for i in range(log_n-1):
        _last = res[-1]
        _next = list(map(lambda e: 2*e^2-1, left_half(_last)))
        res.append(_next)
    return res
"""

def twiddles(log_n):
    if log_n==0:
        return []
    res = []
    domain = symmetric_cfft_permute(evaluation_domain(log_n))[::2]
    res.append(symmetric_cfft_permute(domain))
    for i in range(log_n-1):
        domain = list(map(lambda e: 2*e^2-1, domain[::2]))
        res.append(symmetric_cfft_permute(domain))
    return res



def evaluate_naive(domain, coeffs):
    return [evaluate_at_point(coeffs, p) for p in domain]





def evaluate(domain, coeffs):
    domain_log_n = log2_strict(len(domain))
    coeffs_log_n = log2_strict(len(coeffs))

    assert domain_log_n >= coeffs_log_n

    if coeffs_log_n < domain_log_n:
        coeffs = coeffs + [Fp2.zero()] * (2^(domain_log_n) - len(coeffs))
        coeffs_log_n = domain_log_n

    coeffs = symmetric_cfft_permute(coeffs)
    _twiddles = twiddles(coeffs_log_n)
    _twiddles = list(reversed(_twiddles))



    n=len(coeffs)
    m=1
    for r in range(coeffs_log_n):
        for k in range(0, n, 2*m):
            for j in range(m):
                t = _twiddles[r][j]
                u = coeffs[k+j]
                v = coeffs[k+j+m]*t
                coeffs[k+j] = u + v
                coeffs[k+j+m] = u - v
        m *= 2
    return coeffs




def fft(domain, coeffs):
    def _fft(coeffs):
        if len(coeffs) == 1:
            return coeffs
        
        even = _fft(coeffs[::2])
        odd = _fft(coeffs[1::2])

        n = len(coeffs)
        log_n = log2_strict(n)

        g = circle_subgroup_gen(log_n)
        w = circle_subgroup_gen(log_n+2)
        return [even[i] + (w*g^i)[0] *odd[i] for i in range(n/2)] + [even[i] - (w*g^i)[0] * odd[i] for i in range(n/2)]
    
    
    domain_log_n = log2_strict(len(domain))
    coeffs_log_n = log2_strict(len(coeffs))

    assert domain_log_n >= coeffs_log_n

    if coeffs_log_n < domain_log_n:
        coeffs = coeffs + [Fp.zero()] * (2^(domain_log_n) - len(coeffs))
        coeffs_log_n = domain_log_n

    return _fft(coeffs)
   



def pi(x):
    return 2*x^2-1

def monomial_basis(x, log_n):
    cur = x
    res = [1]
    for i in range(log_n):
        res += [cur * e for e in res]
        cur = pi(cur)
    return res



def evaluate_at_point(coeffs, x):
    return list_prod_sum(coeffs, monomial_basis(x, log2_strict(len(coeffs))))

def evaluate_at_point_lagrange(values, x):
    log_n = log2_strict(len(values))
    domain = evaluation_domain(log_n)
    return sum([v_i*lagrange_basis(x, p, log_n) for (v_i, p) in zip(values,domain)])



def Pi(x):
    return 2*x^2-1

def Pi_n(x, n):
    cur = x
    for i in range(n):
        cur = Pi(cur)
    return cur

def dPi(x):
    return 4*x

# use formula for derivative of composition of functions
# (f^{(n)}(x))' = \Prod_{i=0}^{n-1} f'(f^{(i)}(x))
def dPi_n(x, n):
    p=1
    cur=x
    for i in range(n):
        p*=dPi(cur)
        cur = Pi(cur)
    return p
    

def lagrange_basis(x, x0, log_n):
    if x==x0:
        return 1

    nom=Pi_n(x, log_n)-Pi_n(x0, log_n)
    denom = (x - x0) * dPi_n(x0, log_n)  # Изменено x на x0
    return nom/denom
    



def sample_polynomial(log_n):
    coeffs = [Fp.random_element() for i in range(2^log_n)]
    return coeffs


def symmetric_cfft_permute_test():
    from random import random
    log_n = 5

    values = [randrange(circle_gen_order) for i in range(2^(log_n-1))]
    values_copy = values[:]
    values = symmetric_cfft_permute(values)
    values = symmetric_cfft_permute(values)

    assert np.array_equal(values, values_copy)




def test_lagrange_basis():
    log_n = 5
    domain = evaluation_domain(log_n)
    x = Fp2.random_element()
    assert sum([lagrange_basis(x, p, log_n) for p in domain]) == 1


def test_evaluate_naive():
    log_n = 5
    coeffs = sample_polynomial(log_n)
    domain = evaluation_domain(log_n)
    values = evaluate_naive(domain, coeffs)


    x = Fp3.random_element()
    left = evaluate_at_point(coeffs, x)
    right = evaluate_at_point_lagrange(values, x)
    assert left==right

def test_fft():
    log_n = 5
    coeffs = sample_polynomial(log_n)
    domain = evaluation_domain(log_n)
    values = evaluate(domain, coeffs)

    x = Fp3.random_element()
    left=evaluate_at_point(coeffs, x)
    right=evaluate_at_point_lagrange(values, x)

    assert left==right


def test_fft_and_naive():
    log_n = 8
    coeffs = sample_polynomial(log_n)
    domain = evaluation_domain(log_n)
    
    naive_values = evaluate_naive(domain, coeffs)
    fft_values = evaluate(domain, coeffs)


    assert list_eq(naive_values, fft_values), "Naive and FFT values do not match"


def test_reverse_bits():
    assert reverse_bits(0, 1) == 0
    assert reverse_bits(1, 1) == 1
    assert reverse_bits(2, 2) == 1
    assert reverse_bits(3, 2) == 3
    assert reverse_bits(4, 3) == 1
    assert reverse_bits(5, 3) == 5
    assert reverse_bits(6, 3) == 3
    assert reverse_bits(7, 3) == 7
    assert reverse_bits(8, 4) == 1
    assert reverse_bits(9, 4) == 9
    assert reverse_bits(10, 4) == 5
    assert reverse_bits(11, 4) == 13
    assert reverse_bits(12, 4) == 3
    assert reverse_bits(13, 4) == 11
    assert reverse_bits(14, 4) == 7
    assert reverse_bits(15, 4) == 15


if __name__ == "__main__":
    symmetric_cfft_permute_test()
    test_lagrange_basis()
    test_evaluate_naive()
    test_fft()
    test_fft_and_naive()
    test_reverse_bits()



