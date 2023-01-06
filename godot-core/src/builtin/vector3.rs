/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::real::Real;

impl_vector!(Vector3, crate::builtin::real::Vec3, Real, (x, y, z));
impl_float_vector!(Vector3, Real);
impl_vector_from!(Vector3, Vector3i, Real, (x, y, z));

impl Vector3 {

    /// Left unit vector. Represents the local direction of left, and the global direction of west.
    pub const LEFT: Self = Self::new(-1.0, 0.0, 0.0);

    /// Right unit vector. Represents the local direction of right, and the global direction of east.
    pub const RIGHT: Self = Self::new(1.0, 0.0, 0.0);

    /// Up unit vector.
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);

    /// Down unit vector.
    pub const DOWN: Self = Self::new(0.0, -1.0, 0.0);

    /// Forward unit vector. Represents the local direction of forward, and the global direction of north.
    pub const FORWARD: Self = Self::new(0.0, 0.0, -1.0);

    /// Back unit vector. Represents the local direction of back, and the global direction of south.
    pub const BACK: Self = Self::new(0.0, 0.0, 1.0);
}

impl_vector!(Vector3i, glam::IVec3, i32, (x, y, z));
impl_vector_from!(Vector3i, Vector3, i32, (x, y, z));

// TODO auto-generate this, alongside all the other builtin type's enums

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum Vector3Axis {
    X,
    Y,
    Z,
}

impl GodotFfi for Vector3Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
