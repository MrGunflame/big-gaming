use std::convert::Infallible;

use bytes::{Buf, BufMut};
use glam::Quat;

use super::{Decode, Encode, EofError};

/// The component that needs to be reconstructed.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Component {
    X = 0b00,
    Y = 0b01,
    Z = 0b10,
    W = 0b11,
}

impl Encode for Quat {
    type Error = Infallible;

    #[inline]
    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.x.encode(&mut buf)?;
        self.y.encode(&mut buf)?;
        self.z.encode(&mut buf)?;
        self.w.encode(&mut buf)?;

        // let mut max = self.x.abs();
        // let mut comp = Component::X;
        // if self.y.abs() > max {
        //     max = self.y;
        //     comp = Component::Y;
        // }
        // if self.z.abs() > max {
        //     max = self.z;
        //     comp = Component::Z;
        // }
        // if self.w.abs() > max {
        //     comp = Component::W;
        // }

        // let index: u8 = match comp {
        //     Component::X => 0b00,
        //     Component::Y => 0b01,
        //     Component::Z => 0b10,
        //     Component::W => 0b11,
        // };
        // index.encode(&mut buf)?;

        // let (mut a, mut b, mut c);
        // match comp {
        //     Component::X => {
        //         a = self.y;
        //         b = self.z;
        //         c = self.w;
        //     }
        //     Component::Y => {
        //         a = self.x;
        //         b = self.z;
        //         c = self.w;
        //     }
        //     Component::Z => {
        //         a = self.x;
        //         b = self.y;
        //         c = self.w;
        //     }
        //     Component::W => {
        //         a = self.x;
        //         b = self.y;
        //         c = self.z;
        //     }
        // }

        // If the omitted value is negative we make it positive by negating the
        // entire Quat.
        // if max.is_sign_negative() {
        //     a = -a;
        //     b = -b;
        //     c = -c;
        // }

        // a.encode(&mut buf)?;
        // b.encode(&mut buf)?;
        // c.encode(&mut buf)?;

        Ok(())
    }
}

impl Decode for Quat {
    type Error = EofError;

    #[inline]
    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let x = f32::decode(&mut buf)?;
        let y = f32::decode(&mut buf)?;
        let z = f32::decode(&mut buf)?;
        let w = f32::decode(&mut buf)?;
        Ok(Self::from_xyzw(x, y, z, w))

        // let index = u8::decode(&mut buf)?;
        // let comp = match index & 0b11 {
        //     0b00 => Component::X,
        //     0b01 => Component::Y,
        //     0b10 => Component::Z,
        //     0b11 => Component::W,
        //     _ => unreachable!(),
        // };

        // let a = f32::decode(&mut buf)?;
        // let b = f32::decode(&mut buf)?;
        // let c = f32::decode(&mut buf)?;
        // let d = f32::sqrt(1.0 - a * a - b * b - c * c);

        // let this = match comp {
        //     Component::X => Quat::from_xyzw(d, a, b, c),
        //     Component::Y => Quat::from_xyzw(a, d, b, c),
        //     Component::Z => Quat::from_xyzw(a, b, d, c),
        //     Component::W => Quat::from_xyzw(a, b, c, d),
        // };

        // if this.is_normalized() {
        //     Ok(this)
        // } else {
        //     Ok(this.normalize())
        // }
    }
}

#[cfg(test)]
mod tests {

    use glam::Vec3;

    use super::{Decode, Encode, Quat};

    #[test]
    fn test_quat() {
        for quat in [
            Quat::from_xyzw(0.0, 0.0, 0.0, 1.0),
            Quat::from_xyzw(1.0, 0.0, 0.0, 0.0),
            Quat::from_xyzw(0.0, 1.0, 0.0, 0.0),
            Quat::from_xyzw(0.0, 0.0, 1.0, 0.0),
            Quat::from_axis_angle(Vec3::new(0.0, 0.5, 0.5).normalize(), 0.0),
            Quat::from_axis_angle(Vec3::new(0.4, 0.2, 0.4).normalize(), 0.93),
            Quat::from_axis_angle(Vec3::new(-1.0, 0.0, 0.0), 1.4),
        ] {
            assert!(quat.is_normalized(), "{:?} is not normalized", quat);

            let mut buf = Vec::new();
            quat.encode(&mut buf).unwrap();

            assert_eq!(Quat::decode(&buf[..]).unwrap(), quat);
        }
    }
}
