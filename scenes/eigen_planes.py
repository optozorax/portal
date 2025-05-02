import math, numpy as np

# this is a script to calculate eigenvector for scene `portal_in_portal_cone`

# write your rotation angles here
x = 180
y = 190
z = 179

# reflection usage here
x_reflection = False
y_reflection = False
z_reflection = False

def Rx(d):
    r = math.radians(d); c, s = math.cos(r), math.sin(r)
    return np.array([[1,0,0],[0,c,-s],[0,s,c]], np.float64)

def Ry(d):
    r = math.radians(d); c, s = math.cos(r), math.sin(r)
    return np.array([[c,0,s],[0,1,0],[-s,0,c]], np.float64)

def Rz(d):
    r = math.radians(d); c, s = math.cos(r), math.sin(r)
    return np.array([[c,-s,0],[s, c,0],[0,0,1]], np.float64)

def reflection_matrix(xr, yr, zr):
    return np.diag([
        -1 if xr else 1,
        -1 if yr else 1,
        -1 if zr else 1
    ])

R = Rx(x) @ Ry(y) @ Rz(z)
R = reflection_matrix(x_reflection, y_reflection, z_reflection) @ R

eigvals, eigvecs = np.linalg.eig(R)

print(f"for angles: {x} {y} {z}")
print(f"for rotations: {x_reflection} {y_reflection} {z_reflection}")
print()

for idx in range(3):
    lam = eigvals[idx]
    v = eigvecs[:, idx]

    print(f"λ_{idx} = {lam}")
    print(f"full eigenvector_{idx} = ({v[0]}, {v[1]}, {v[2]})")

    v = v.real
    print(f"real eigenvector_{idx} = ({v[0]}, {v[1]}, {v[2]})")
    print()
