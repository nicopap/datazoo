//! Packed structs for compact and user-controlled representation of complex data.
#![warn(clippy::pedantic, clippy::nursery, missing_docs)]
#![allow(
    clippy::use_self,
    clippy::module_name_repetitions,
    clippy::redundant_pub_crate
)]

pub use arbitrary_int::*;
pub use dz_bitflags_macros::{Flag, Flags};

pub trait Bitsized {
    const BIT_SIZE: u32;
}
pub trait Flags: Bitsized {
    type Packed: Bitsized;
    type Reader: FlagsReader<Self::Packed>;

    fn from_packed(packed: Self::Packed) -> Self;
    fn into_packed(self) -> Self::Packed;
}
/// A non-deserialized [`Flags::Packed`] reader.
///
/// The [`Flags`] macro automatically creates a `FooReader` struct that can
/// reads individual fields from the packed representation without having to read
/// other fields.
pub trait FlagsReader<P> {
    fn from_packed(packed: P) -> Self;
}

impl<P> FlagsReader<P> for () {
    fn from_packed(_: P) {}
}

impl Bitsized for bool {
    const BIT_SIZE: u32 = 1;
}
macro_rules! impl_flag {
    (@unint $($tys:ty),* $(,)?) => {
        $(impl<const S: usize> Bitsized for Uint<$tys, S> {
            const BIT_SIZE: u32 = S as u32;
        })*
    };
    (@rustint $($tys:ty),* $(,)?) => {
        $(impl Bitsized for $tys {
            const BIT_SIZE: u32 = $tys::BITS;
        })*
        $(impl<const N: usize> Bitsized for [$tys; N] {
            const BIT_SIZE: u32 = $tys::BITS * (N as u32);
        })*
    }
}
impl_flag![@unint u8, u16, u32, u64];
impl_flag![@rustint u8, u16, u32, u64];
