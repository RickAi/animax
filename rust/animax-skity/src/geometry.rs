// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use animax_model::{PointF, RectF};

/// Column-major 4x4 transformation matrix matching `skity::Matrix`.
/// `m[col][row]`: column 0..3, row 0..3.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub m: [[f32; 4]; 4],
}

impl Default for Matrix {
    fn default() -> Self {
        Self::identity()
    }
}

impl Matrix {
    #[inline]
    pub const fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    #[inline]
    pub const fn translate(dx: f32, dy: f32) -> Self {
        Self::translate_3d(dx, dy, 0.0)
    }

    #[inline]
    pub const fn translate_3d(dx: f32, dy: f32, dz: f32) -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [dx, dy, dz, 1.0],
            ],
        }
    }

    #[inline]
    pub const fn scale(sx: f32, sy: f32) -> Self {
        Self::scale_3d(sx, sy, 1.0)
    }

    #[inline]
    pub const fn scale_3d(sx: f32, sy: f32, sz: f32) -> Self {
        Self {
            m: [
                [sx, 0.0, 0.0, 0.0],
                [0.0, sy, 0.0, 0.0],
                [0.0, 0.0, sz, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    #[inline]
    pub const fn skew(kx: f32, ky: f32) -> Self {
        Self {
            m: [
                [1.0, ky, 0.0, 0.0],
                [kx, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    #[inline]
    pub fn rotate_deg(deg: f32) -> Self {
        Self::rotate_rad(deg.to_radians())
    }

    #[inline]
    pub fn rotate_rad(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            m: [
                [c, s, 0.0, 0.0],
                [-s, c, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    #[inline]
    pub fn from_9(v: &[f32; 9]) -> Self {
        // Row-major 3x3: [sx, kx, tx, ky, sy, ty, p0, p1, p2]
        Self {
            m: [
                [v[0], v[3], 0.0, v[6]],
                [v[1], v[4], 0.0, v[7]],
                [0.0, 0.0, 1.0, 0.0],
                [v[2], v[5], 0.0, v[8]],
            ],
        }
    }

    #[inline]
    pub fn get_9(&self) -> [f32; 9] {
        [
            self.m[0][0],
            self.m[1][0],
            self.m[3][0],
            self.m[0][1],
            self.m[1][1],
            self.m[3][1],
            self.m[0][3],
            self.m[1][3],
            self.m[3][3],
        ]
    }

    #[inline]
    pub fn is_identity(&self) -> bool {
        *self == Self::identity()
    }

    #[inline]
    pub fn reset(&mut self) {
        *self = Self::identity();
    }

    /// Matrix multiplication `self * rhs` (column-major).
    #[inline]
    pub fn mul(&self, rhs: &Matrix) -> Matrix {
        let mut out = [[0.0f32; 4]; 4];
        for col in 0..4 {
            for row in 0..4 {
                out[col][row] = self.m[0][row] * rhs.m[col][0]
                    + self.m[1][row] * rhs.m[col][1]
                    + self.m[2][row] * rhs.m[col][2]
                    + self.m[3][row] * rhs.m[col][3];
            }
        }
        Matrix { m: out }
    }

    #[inline]
    pub fn pre_concat(&mut self, other: &Matrix) {
        *self = self.mul(other);
    }

    #[inline]
    pub fn post_concat(&mut self, other: &Matrix) {
        *self = other.mul(self);
    }

    #[inline]
    pub fn pre_translate(&mut self, dx: f32, dy: f32) {
        self.pre_concat(&Self::translate(dx, dy));
    }

    #[inline]
    pub fn pre_translate_3d(&mut self, dx: f32, dy: f32, dz: f32) {
        self.pre_concat(&Self::translate_3d(dx, dy, dz));
    }

    #[inline]
    pub fn pre_scale(&mut self, sx: f32, sy: f32) {
        self.pre_concat(&Self::scale(sx, sy));
    }

    #[inline]
    pub fn pre_scale_3d(&mut self, sx: f32, sy: f32, sz: f32) {
        self.pre_concat(&Self::scale_3d(sx, sy, sz));
    }

    #[inline]
    pub fn pre_rotate(&mut self, deg: f32) {
        self.pre_concat(&Self::rotate_deg(deg));
    }

    #[inline]
    pub fn pre_rotate_at(&mut self, deg: f32, px: f32, py: f32) {
        self.pre_translate(px, py);
        self.pre_rotate(deg);
        self.pre_translate(-px, -py);
    }

    #[inline]
    pub fn pre_rotate_xyz(&mut self, x_deg: f32, y_deg: f32, z_deg: f32) {
        let q = Quaternion::euler_to_matrix(
            x_deg.to_radians(),
            y_deg.to_radians(),
            z_deg.to_radians(),
        );
        self.pre_concat(&q);
    }

    #[inline]
    pub fn pre_skew(&mut self, kx: f32, ky: f32) {
        self.pre_concat(&Self::skew(kx, ky));
    }

    #[inline]
    pub fn map_xy(&self, x: f32, y: f32) -> (f32, f32) {
        let nx = self.m[0][0] * x + self.m[1][0] * y + self.m[3][0];
        let ny = self.m[0][1] * x + self.m[1][1] * y + self.m[3][1];
        let nw = self.m[0][3] * x + self.m[1][3] * y + self.m[3][3];
        if nw != 0.0 && nw != 1.0 {
            (nx / nw, ny / nw)
        } else {
            (nx, ny)
        }
    }

    #[inline]
    pub fn map_point(&self, p: PointF) -> PointF {
        let (x, y) = self.map_xy(p.x, p.y);
        PointF::new(x, y)
    }

    pub fn map_rect(&self, r: RectF) -> RectF {
        let corners = [
            self.map_xy(r.left, r.top),
            self.map_xy(r.right, r.top),
            self.map_xy(r.right, r.bottom),
            self.map_xy(r.left, r.bottom),
        ];
        let mut left = f32::INFINITY;
        let mut top = f32::INFINITY;
        let mut right = f32::NEG_INFINITY;
        let mut bottom = f32::NEG_INFINITY;
        for (x, y) in corners {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
        RectF::new(left, top, right, bottom)
    }

    /// Inverts the Z=0 2D projective/affine plane (matches `Matrix::InvertZ0Plane`).
    pub fn invert_z0_plane(&self) -> Option<Matrix> {
        let a = self.m[0][0] as f64;
        let b = self.m[1][0] as f64;
        let c = self.m[3][0] as f64;
        let d = self.m[0][1] as f64;
        let e = self.m[1][1] as f64;
        let f = self.m[3][1] as f64;
        let g = self.m[0][3] as f64;
        let h = self.m[1][3] as f64;
        let i = self.m[3][3] as f64;

        let c00 = e * i - f * h;
        let c01 = -(d * i - f * g);
        let c02 = d * h - e * g;
        let det = a * c00 + b * c01 + c * c02;
        if det.abs() < 1e-12 {
            return None;
        }
        let inv = 1.0 / det;
        let c10 = -(b * i - c * h);
        let c11 = a * i - c * g;
        let c12 = -(a * h - b * g);
        let c20 = b * f - c * e;
        let c21 = -(a * f - c * d);
        let c22 = a * e - b * d;

        Some(Matrix::from_9(&[
            (c00 * inv) as f32,
            (c10 * inv) as f32,
            (c20 * inv) as f32,
            (c01 * inv) as f32,
            (c11 * inv) as f32,
            (c21 * inv) as f32,
            (c02 * inv) as f32,
            (c12 * inv) as f32,
            (c22 * inv) as f32,
        ]))
    }

    /// Average scale factor matching `lynx::animax::Matrix::GetScale`.
    pub fn get_scale(&self) -> f32 {
        let inv_sqrt_2 = std::f32::consts::FRAC_1_SQRT_2;
        let (x0, y0) = self.map_xy(0.0, 0.0);
        let (x1, y1) = self.map_xy(inv_sqrt_2, inv_sqrt_2);
        (x1 - x0).hypot(y1 - y0)
    }

    /// Converts the 2D affine component into a `tiny_skia::Transform`.
    #[inline]
    pub fn to_tiny_skia(&self) -> tiny_skia::Transform {
        tiny_skia::Transform::from_row(
            self.m[0][0],
            self.m[0][1],
            self.m[1][0],
            self.m[1][1],
            self.m[3][0],
            self.m[3][1],
        )
    }
}

/// Quaternion helper matching `skity::Quaternion`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    pub fn from_euler(x_rad: f32, y_rad: f32, z_rad: f32) -> Self {
        let (sx, cx) = (x_rad * 0.5).sin_cos();
        let (sy, cy) = (y_rad * 0.5).sin_cos();
        let (sz, cz) = (z_rad * 0.5).sin_cos();
        Self {
            x: sx * cy * cz - cx * sy * sz,
            y: cx * sy * cz + sx * cy * sz,
            z: cx * cy * sz - sx * sy * cz,
            w: cx * cy * cz + sx * sy * sz,
        }
    }

    pub fn to_matrix(self) -> Matrix {
        let xx = self.x * self.x;
        let yy = self.y * self.y;
        let zz = self.z * self.z;
        let xy = self.x * self.y;
        let xz = self.x * self.z;
        let yz = self.y * self.z;
        let wx = self.w * self.x;
        let wy = self.w * self.y;
        let wz = self.w * self.z;

        Matrix {
            m: [
                [1.0 - 2.0 * (yy + zz), 2.0 * (xy + wz), 2.0 * (xz - wy), 0.0],
                [2.0 * (xy - wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz + wx), 0.0],
                [2.0 * (xz + wy), 2.0 * (yz - wx), 1.0 - 2.0 * (xx + yy), 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn euler_to_matrix(x_rad: f32, y_rad: f32, z_rad: f32) -> Matrix {
        Self::from_euler(x_rad, y_rad, z_rad).to_matrix()
    }
}

/// 3D perspective camera helper matching `skity::Camera`.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: [f32; 3],
    pub look_at: [f32; 3],
    pub up: [f32; 3],
    pub fov_deg: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, -576.0],
            look_at: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov_deg: 45.0,
        }
    }
}

impl Camera {
    pub fn perspective_matrix(distance: f32) -> Matrix {
        let mut mat = Matrix::identity();
        if distance.abs() > 1e-4 {
            mat.m[2][3] = -1.0 / distance;
        }
        mat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_translate_scale_invert() {
        let mut m = Matrix::identity();
        m.pre_translate(10.0, 20.0);
        m.pre_scale(2.0, 3.0);
        let p = m.map_point(PointF::new(5.0, 4.0));
        assert!((p.x - 20.0).abs() < 1e-5);
        assert!((p.y - 32.0).abs() < 1e-5);

        let inv = m.invert_z0_plane().unwrap();
        let back = inv.map_point(p);
        assert!((back.x - 5.0).abs() < 1e-4);
        assert!((back.y - 4.0).abs() < 1e-4);
    }

    #[test]
    fn matrix_rotate_90() {
        let m = Matrix::rotate_deg(90.0);
        let p = m.map_point(PointF::new(1.0, 0.0));
        assert!(p.x.abs() < 1e-5);
        assert!((p.y - 1.0).abs() < 1e-5);
    }
}
