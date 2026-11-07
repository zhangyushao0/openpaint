//! Vectors in space (S93, S94): the world the eye sees boxes, solids and objects in (y up, z away
//! from the eye).

pub(crate) type Vector = [f64; 3];

pub(crate) fn add(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|k| a[k] + b[k])
}

pub(crate) fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|k| a[k] - b[k])
}

pub(crate) fn times(a: Vector, by: f64) -> Vector {
    a.map(|v| v * by)
}

pub(crate) fn dot(a: Vector, b: Vector) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}

pub(crate) fn cross(a: Vector, b: Vector) -> Vector {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn norm(a: Vector) -> f64 {
    dot(a, a).sqrt()
}

/// `a` made one long; `a` itself when it has no length.
pub(crate) fn unit(a: Vector) -> Vector {
    let long = norm(a);
    if long > 0.0 { times(a, 1.0 / long) } else { a }
}

/// Two unit vectors square to `a` (one long) and to each other.
pub(crate) fn across(a: Vector) -> [Vector; 2] {
    let other = if a[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let first = unit(cross(a, other));
    [first, cross(a, first)]
}
