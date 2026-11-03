import math
class V(tuple):
    def __new__(c,it): return tuple.__new__(c,[float(a) for a in it])
    def __add__(a,b): return V(x+y for x,y in zip(a,b))
    def __sub__(a,b): return V(x-y for x,y in zip(a,b))
    def __neg__(a): return V(-x for x in a)
    def __mul__(a,k): return V(x*k for x in a)
    __rmul__=__mul__
    def __matmul__(a,b): return sum(x*y for x,y in zip(a,b))
class NP:
    @staticmethod
    def eye(n): return [V([1 if i==j else 0 for j in range(n)]) for i in range(n)]
    @staticmethod
    def array(v,dtype=None): return V(v)
    @staticmethod
    def column_stack(cols): return cols
    @staticmethod
    def cross(a,b): return V((a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]))
    @staticmethod
    def allclose(a,b): return all(abs(x-y)<1e-9 for x,y in zip(a,b))
    class linalg:
        @staticmethod
        def norm(a): return math.sqrt(a@a)
np=NP
