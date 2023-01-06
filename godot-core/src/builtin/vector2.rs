/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::real::Real;

impl_vector!(Vector2, crate::builtin::real::Vec2, Real, (x, y));
impl_float_vector!(Vector2, Real);
impl_vector_from!(Vector2, Vector2i, Real, (x, y));

impl Vector2 {

    /// Left unit vector. Represents the direction of left.
    pub const LEFT: Self = Self::new(-1.0, 0.0);

    /// Right unit vector. Represents the direction of right.
    pub const RIGHT: Self = Self::new(1.0, 0.0);

    /// Up unit vector. Y is down in 2D, so this vector points -Y.
    pub const UP: Self = Self::new(0.0, -1.0);

    /// Down unit vector. Y is down in 2D, so this vector points +Y.
    pub const DOWN: Self = Self::new(0.0, 1.0);

    pub fn rotated(self, angle: f32) -> Self {
        glam::Affine2::from_angle(angle).transform_vector2(self.into()).into()
    }
}

impl_vector!(Vector2i, glam::IVec2, i32, (x, y));
impl_vector_from!(Vector2i, Vector2, i32, (x, y));
