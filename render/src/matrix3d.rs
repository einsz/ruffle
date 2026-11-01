use crate::matrix::Matrix;
use swf::Twips;

/// The transformation matrix for 3D used by Flash display objects.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Matrix3D {
    /// 4x4 matrix elements.
    pub raw_data: [f32; 16],
}

impl Matrix3D {
    pub const IDENTITY: Self = Self {
        raw_data: [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ],
    };

    pub fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            raw_data: [
                x, 0.0, 0.0, 0.0, //
                0.0, y, 0.0, 0.0, //
                0.0, 0.0, z, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn translate(x: f32, y: f32, z: f32) -> Self {
        Self {
            raw_data: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                x, y, z, 1.0,
            ],
        }
    }

    pub fn from_matrix(matrix: Matrix) -> Self {
        Self {
            raw_data: [
                // 1st column
                matrix.a,
                matrix.b,
                0.0,
                0.0,
                // 2nd column
                matrix.c,
                matrix.d,
                0.0,
                0.0,
                // 3rd column
                0.0,
                0.0,
                1.0,
                0.0,
                // 4th column
                matrix.tx.to_pixels() as f32,
                matrix.ty.to_pixels() as f32,
                0.0,
                1.0,
            ],
        }
    }

    /// A rotation about the X, then the Y, then the Z axis, in radians.
    pub fn euler_rotation(x: f64, y: f64, z: f64) -> Self {
        let (sx, cx) = x.sin_cos();
        let (sy, cy) = y.sin_cos();
        let (sz, cz) = z.sin_cos();
        Self {
            #[rustfmt::skip]
            raw_data: [
                cy * cz, cy * sz, -sy, 0.0,
                sx * sy * cz - cx * sz, sx * sy * sz + cx * cz, sx * cy, 0.0,
                cx * sy * cz + sx * sz, cx * sy * sz - sx * cz, cx * cy, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ]
            .map(|f| f as f32),
        }
    }

    /// Splits the linear part of this matrix into a rotation (as used by
    /// `euler_rotation`) and a scale, as display objects do.
    ///
    /// Unlike `Matrix3D.decompose`, this orthogonalizes the axes, so a skew
    /// shows up as a smaller scale rather than a longer axis.
    pub fn decompose_rotation_scale(&self) -> ([f64; 3], [f64; 3]) {
        fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
            a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
        }
        fn sub_projection(a: [f64; 3], unit: [f64; 3]) -> [f64; 3] {
            let d = dot(a, unit);
            [a[0] - d * unit[0], a[1] - d * unit[1], a[2] - d * unit[2]]
        }
        fn normalize(a: [f64; 3]) -> (f64, [f64; 3]) {
            let length = dot(a, a).sqrt();
            if length == 0.0 {
                (0.0, a)
            } else {
                (length, a.map(|f| f / length))
            }
        }

        let m = self.raw_data.map(f64::from);
        let x_axis = [m[0], m[1], m[2]];
        let y_axis = [m[4], m[5], m[6]];
        let z_axis = [m[8], m[9], m[10]];

        let (scale_x, x_unit) = normalize(x_axis);
        let (scale_y, y_unit) = normalize(sub_projection(y_axis, x_unit));
        let (mut scale_z, mut z_unit) =
            normalize(sub_projection(sub_projection(z_axis, x_unit), y_unit));

        let determinant = x_axis[0] * (y_axis[1] * z_axis[2] - y_axis[2] * z_axis[1])
            - x_axis[1] * (y_axis[0] * z_axis[2] - y_axis[2] * z_axis[0])
            + x_axis[2] * (y_axis[0] * z_axis[1] - y_axis[1] * z_axis[0]);
        if determinant < 0.0 {
            scale_z = -scale_z;
            z_unit = z_unit.map(|f| -f);
        }

        let rotation_y = (-x_unit[2]).clamp(-1.0, 1.0).asin();
        let (rotation_x, rotation_z) = if x_unit[2].abs() < 1.0 {
            (y_unit[2].atan2(z_unit[2]), x_unit[1].atan2(x_unit[0]))
        } else {
            // Gimbal lock: rotationY is 90 or -90 degrees, which flips the
            // sign of rotationX in the Y axis.
            ((-x_unit[2] * y_unit[0]).atan2(y_unit[1]), 0.0)
        };

        (
            [rotation_x, rotation_y, rotation_z],
            [scale_x, scale_y, scale_z],
        )
    }

    pub fn to_matrix(self) -> Matrix {
        Matrix {
            a: self.raw_data[0],
            b: self.raw_data[1],
            c: self.raw_data[4],
            d: self.raw_data[5],
            tx: Twips::from_pixels(self.raw_data[12] as f64),
            ty: Twips::from_pixels(self.raw_data[13] as f64),
        }
    }

    pub fn transpose_in_place(&mut self) {
        self.raw_data.swap(1, 4);
        self.raw_data.swap(2, 8);
        self.raw_data.swap(3, 12);
        self.raw_data.swap(6, 9);
        self.raw_data.swap(7, 13);
        self.raw_data.swap(11, 14);
    }

    pub fn determinant(&self) -> f32 {
        let m = &self.raw_data;

        /// The determinant of a 3x3 matrix, stored in column-major order.
        fn determinant_3x3(m: [f32; 9]) -> f32 {
            m[0] * (m[4] * m[8] - m[7] * m[5]) - m[3] * (m[1] * m[8] - m[7] * m[2])
                + m[6] * (m[1] * m[5] - m[4] * m[2])
        }

        // When projection is 0, Flash ignores translation, suggesting there is
        // a fast path here.
        if m[3] == 0.0 && m[7] == 0.0 && m[11] == 0.0 {
            // This is equivalent to doing a Laplace expansion over the last
            // column, but considering only the last row is non-zero, we can
            // skip other rows.
            //
            // Usually we'd calculate the determinant of the inner 3x3 matrix
            // and multiply by the last element, but Flash appears to factor the
            // last element into the formula (observable when it's +-Infinity).
            determinant_3x3([
                m[0],
                m[1],
                m[2] * m[15],
                m[4],
                m[5],
                m[6] * m[15],
                m[8],
                m[9],
                m[10] * m[15],
            ])
        } else {
            // Based on the condition above, it looks like we should do the
            // Laplace expansion over the last column, but that's not what
            // happens. This is observable when using +-Infinity in a dense
            // matrix; infinities are preserved only in the first row, which
            // means that no two infinities are subtracted in the expression,
            // suggesting we should do the Laplace expansion over that row.
            let det0 = determinant_3x3([m[5], m[6], m[7], m[9], m[10], m[11], m[13], m[14], m[15]]);
            let det4 = determinant_3x3([m[1], m[2], m[3], m[9], m[10], m[11], m[13], m[14], m[15]]);
            let det8 = determinant_3x3([m[1], m[2], m[3], m[5], m[6], m[7], m[13], m[14], m[15]]);
            let det12 = determinant_3x3([m[1], m[2], m[3], m[5], m[6], m[7], m[9], m[10], m[11]]);
            m[0] * det0 - m[4] * det4 + m[8] * det8 - m[12] * det12
        }
    }

    pub fn invert(&self) -> Option<Self> {
        let d = self.determinant();
        let invertible = d.abs() > 0.00000000001;

        if !invertible {
            return None;
        }

        let d = 1.0 / d;

        let m = self.raw_data;
        let m11 = m[0];
        let m21 = m[4];
        let m31 = m[8];
        let m41 = m[12];
        let m12 = m[1];
        let m22 = m[5];
        let m32 = m[9];
        let m42 = m[13];
        let m13 = m[2];
        let m23 = m[6];
        let m33 = m[10];
        let m43 = m[14];
        let m14 = m[3];
        let m24 = m[7];
        let m34 = m[11];
        let m44 = m[15];

        #[rustfmt::skip]
        let raw_data = [
             d * (m22 * (m33 * m44 - m43 * m34) - m32 * (m23 * m44 - m43 * m24) + m42 * (m23 * m34 - m33 * m24)),
            -d * (m12 * (m33 * m44 - m43 * m34) - m32 * (m13 * m44 - m43 * m14) + m42 * (m13 * m34 - m33 * m14)),
             d * (m12 * (m23 * m44 - m43 * m24) - m22 * (m13 * m44 - m43 * m14) + m42 * (m13 * m24 - m23 * m14)),
            -d * (m12 * (m23 * m34 - m33 * m24) - m22 * (m13 * m34 - m33 * m14) + m32 * (m13 * m24 - m23 * m14)),
            -d * (m21 * (m33 * m44 - m43 * m34) - m31 * (m23 * m44 - m43 * m24) + m41 * (m23 * m34 - m33 * m24)),
             d * (m11 * (m33 * m44 - m43 * m34) - m31 * (m13 * m44 - m43 * m14) + m41 * (m13 * m34 - m33 * m14)),
            -d * (m11 * (m23 * m44 - m43 * m24) - m21 * (m13 * m44 - m43 * m14) + m41 * (m13 * m24 - m23 * m14)),
             d * (m11 * (m23 * m34 - m33 * m24) - m21 * (m13 * m34 - m33 * m14) + m31 * (m13 * m24 - m23 * m14)),
             d * (m21 * (m32 * m44 - m42 * m34) - m31 * (m22 * m44 - m42 * m24) + m41 * (m22 * m34 - m32 * m24)),
            -d * (m11 * (m32 * m44 - m42 * m34) - m31 * (m12 * m44 - m42 * m14) + m41 * (m12 * m34 - m32 * m14)),
             d * (m11 * (m22 * m44 - m42 * m24) - m21 * (m12 * m44 - m42 * m14) + m41 * (m12 * m24 - m22 * m14)),
            -d * (m11 * (m22 * m34 - m32 * m24) - m21 * (m12 * m34 - m32 * m14) + m31 * (m12 * m24 - m22 * m14)),
            -d * (m21 * (m32 * m43 - m42 * m33) - m31 * (m22 * m43 - m42 * m23) + m41 * (m22 * m33 - m32 * m23)),
             d * (m11 * (m32 * m43 - m42 * m33) - m31 * (m12 * m43 - m42 * m13) + m41 * (m12 * m33 - m32 * m13)),
            -d * (m11 * (m22 * m43 - m42 * m23) - m21 * (m12 * m43 - m42 * m13) + m41 * (m12 * m23 - m22 * m13)),
             d * (m11 * (m22 * m33 - m32 * m23) - m21 * (m12 * m33 - m32 * m13) + m31 * (m12 * m23 - m22 * m13)),
        ];

        Some(Self { raw_data })
    }

    pub fn multiply(&self, rhs: &Self) -> Self {
        let lhs = &self.raw_data;
        let rhs = &rhs.raw_data;
        let mut result = [0.0; 16];

        for column in 0..4 {
            for row in 0..4 {
                result[column * 4 + row] = lhs[row] * rhs[column * 4]
                    + lhs[4 + row] * rhs[column * 4 + 1]
                    + lhs[8 + row] * rhs[column * 4 + 2]
                    + lhs[12 + row] * rhs[column * 4 + 3];
            }
        }

        Self { raw_data: result }
    }

    pub fn tz(&self) -> f32 {
        self.raw_data[14]
    }
    pub fn set_tz(&mut self, tz: f32) {
        self.raw_data[14] = tz;
    }
}
