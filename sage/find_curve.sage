import sys


p = 2^31-1
q = p^8
Fp8.<x> = GF(q)


def check_num_points(num_points):
    cofactor = 8
    twist_cofactor = 4

    twist_num_points = 2*(q+1) - num_points

    if num_points % cofactor != 0:
        return False

    if twist_num_points % twist_cofactor != 0:
        return False

    if not (num_points // cofactor).is_prime():
        return False
    
    if not (twist_num_points // twist_cofactor).is_prime():
        return False
    
    return True

    

def check_parameters(A):
    A2 = A^2
    if A2 - 4 == 0:
        return False
    
    if (A2 - 4).is_square():
        return False
    
    if not (A+2).is_square():
        return False

    return True


def count_points_montgomery_curve(A):
    E = EllipticCurve(Fp8, [0, A, 0, 1, 0])
    num_points = E.cardinality()
    return num_points


def main(offset, nthreads):
    max_iter = 10000000

    for i in range(offset, max_iter, nthreads):
        A = i + x  # Assuming 'x' is defined elsewhere in your actual code
        if not check_parameters(A):
            continue
        num_points = count_points_montgomery_curve(A)

        if check_num_points(num_points):
            print(f"Found: A={A} order={num_points} cofactor={num_points//8}")
            break

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python script.py <offset> <nthreads>")
        sys.exit(1)
    
    offset = int(sys.argv[1])
    nthreads = int(sys.argv[2])
    main(offset, nthreads)