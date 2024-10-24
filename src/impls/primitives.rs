use crate::{de, Deserialize, Result, Serialize, Tape, Write};
use std::convert::TryFrom;

impl Serialize for bool {
    #[inline]
    fn json_write<W: Write>(&self, writer: &mut W) -> Result {
        match *self {
            true => writer.write_all(b"true"),
            false => writer.write_all(b"false"),
        }
    }
}

impl<'input> Deserialize<'input> for bool {
    #[inline]
    fn from_tape(tape: &mut Tape<'input>) -> de::Result<bool> {
        if let Some(simd_json::Node::Static(simd_json::StaticNode::Bool(r))) = tape.next() {
            Ok(r)
        } else {
            Err(de::Error::expected_boolean())
        }
    }
}

macro_rules! itoa {
    ($t:ty) => {
        impl Serialize for $t {
            #[inline]
            fn json_write<W>(&self, writer: &mut W) -> std::io::Result<()>
            where
                W: Write,
            {
                let mut buffer = itoa::Buffer::new();
                let s = buffer.format(*self);
                writer.write_all(s.as_bytes())
            }
        }

        impl<'input> Deserialize<'input> for $t {
            #[inline]
            fn from_tape(tape: &mut Tape<'input>) -> de::Result<Self>
            where
                Self: std::marker::Sized + 'input,
            {
                match tape.next() {
                    Some(simd_json::Node::Static(simd_json::StaticNode::I64(i))) => {
                        <$t>::try_from(i).map_err(de::Error::from)
                    }
                    Some(simd_json::Node::Static(simd_json::StaticNode::U64(i))) => {
                        <$t>::try_from(i).map_err(de::Error::from)
                    }
                    #[cfg(feature = "128bit")]
                    Some(simd_json::Node::Static(simd_json::StaticNode::U128(i))) => {
                        <$t>::try_from(i).map_err(de::Error::from)
                    }
                    #[cfg(feature = "128bit")]
                    Some(simd_json::Node::Static(simd_json::StaticNode::I128(i))) => {
                        <$t>::try_from(i).map_err(de::Error::from)
                    }
                    _ => Err(de::Error::expected_integer()),
                }
            }
        }
    };
}

itoa!(i8);
itoa!(u8);
itoa!(i16);
itoa!(u16);
itoa!(i32);
itoa!(u32);
itoa!(i64);
itoa!(u64);
itoa!(usize);
itoa!(i128);
itoa!(u128);

macro_rules! ryu {
    ($t:ty) => {
        impl Serialize for $t {
            #[inline]
            fn json_write<W>(&self, writer: &mut W) -> std::io::Result<()>
            where
                W: Write,
            {
                let mut buffer = ryu::Buffer::new();
                let s = buffer.format_finite(*self);
                writer.write_all(s.as_bytes())
            }
        }
    };
}
ryu!(f64);
ryu!(f32);

impl<'input> Deserialize<'input> for f64 {
    #[inline]
    fn from_tape(tape: &mut Tape<'input>) -> de::Result<Self>
    where
        Self: Sized + 'input,
    {
        match tape.next() {
            Some(simd_json::Node::Static(simd_json::StaticNode::F64(i))) => Ok(i),
            Some(simd_json::Node::Static(simd_json::StaticNode::I64(i))) => Ok(i as f64),
            Some(simd_json::Node::Static(simd_json::StaticNode::U64(i))) => Ok(i as f64),
            #[cfg(feature = "128bit")]
            Some(simd_json::Node::Static(simd_json::StaticNode::U128(i))) => Ok(i as f64),
            #[cfg(feature = "128bit")]
            Some(simd_json::Node::Static(simd_json::StaticNode::I128(i))) => Ok(i as f64),
            _ => Err(de::Error::expected_float()),
        }
    }
}

impl<'input> Deserialize<'input> for f32 {
    #[inline]
    fn from_tape(tape: &mut Tape<'input>) -> de::Result<Self>
    where
        Self: Sized + 'input,
    {
        match tape.next() {
            Some(simd_json::Node::Static(simd_json::StaticNode::F64(i))) => Ok(i as f32),
            Some(simd_json::Node::Static(simd_json::StaticNode::I64(i))) => Ok(i as f32),
            Some(simd_json::Node::Static(simd_json::StaticNode::U64(i))) => Ok(i as f32),
            #[cfg(feature = "128bit")]
            Some(simd_json::Node::Static(simd_json::StaticNode::U128(i))) => Ok(i as f32),
            #[cfg(feature = "128bit")]
            Some(simd_json::Node::Static(simd_json::StaticNode::I128(i))) => Ok(i as f32),
            _ => Err(de::Error::expected_float()),
        }
    }
}
