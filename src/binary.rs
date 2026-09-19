//! Replica's bounded, typed metadata and record-stream codec (R3BIN v1).
//! Existing model/corpus/event codecs keep their own schemas. This codec replaces
//! text serialization at IPC, diagnostic and standalone metadata boundaries.
use crate::codec::{Reader, put_bytes, put_varint};
use serde::{Deserialize, Serialize, de, ser};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt,
    io::{Read, Write},
    ops::{Index, IndexMut},
};

const MAGIC: &[u8; 8] = b"R3BIN\0\0\0";
const HEADER: usize = 52;
pub const MAX_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_FRAME_BYTES: usize = MAX_BYTES + HEADER;
const MAX_ITEMS: usize = 1_000_000;
const MAX_DEPTH: usize = 64;
#[derive(Debug, thiserror::Error)]
#[error("Replica binary: {0}")]
pub struct Error(String);
type Result<T> = std::result::Result<T, Error>;
impl ser::Error for Error {
    fn custom<T: fmt::Display>(v: T) -> Self {
        Self(v.to_string())
    }
}
impl de::Error for Error {
    fn custom<T: fmt::Display>(v: T) -> Self {
        Self(v.to_string())
    }
}
impl From<std::io::Error> for Error {
    fn from(v: std::io::Error) -> Self {
        Self(v.to_string())
    }
}
impl From<crate::Error> for Error {
    fn from(v: crate::Error) -> Self {
        Self(v.to_string())
    }
}
fn bad(s: &str) -> Error {
    Error(s.into())
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}
impl Value {
    /// Canonical uncompressed bytes, without rebuilding an already owned value tree.
    pub fn to_vec(&self) -> Result<Vec<u8>> {
        encode_frame(self)
    }
    /// Storage representation; canonical hashing and IPC continue to use `to_vec`.
    pub fn to_storage_vec(&self) -> Result<Vec<u8>> {
        let raw = self.to_vec()?;
        if raw.len() < 4096 || !matches!(self, Self::Array(_) | Self::Object(_)) {
            return Ok(raw);
        }
        let body = &raw[HEADER..];
        let compressed = zstd::bulk::compress(body, 1)?;
        if compressed.len() + 8 > body.len() - body.len() / 8 {
            return Ok(raw);
        }
        let mut out = Vec::with_capacity(HEADER + 8 + compressed.len());
        out.extend_from_slice(&raw[..HEADER]);
        out[10..12].copy_from_slice(&1u16.to_le_bytes());
        out[12..20].copy_from_slice(&((8 + compressed.len()) as u64).to_le_bytes());
        out.extend_from_slice(&(body.len() as u64).to_le_bytes());
        out.extend_from_slice(&compressed);
        Ok(out)
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }
    pub fn is_number(&self) -> bool {
        matches!(
            self,
            Self::U64(_) | Self::I64(_) | Self::F32(_) | Self::F64(_)
        )
    }
    pub fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }
    pub fn is_f64(&self) -> bool {
        matches!(self, Self::F32(_) | Self::F64(_))
    }
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::U64(v) => Some(*v),
            Self::I64(v) => (*v).try_into().ok(),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(v) => Some(*v),
            Self::U64(v) => (*v).try_into().ok(),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::F64(v) => Some(*v),
            Self::F32(v) => Some(f64::from(*v)),
            Self::U64(v) => Some(*v as f64),
            Self::I64(v) => Some(*v as f64),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        if let Self::Object(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_object_mut(&mut self) -> Option<&mut BTreeMap<String, Value>> {
        if let Self::Object(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn get<K: Key>(&self, k: K) -> Option<&Value> {
        k.get(self)
    }
    pub fn get_mut<K: Key>(&mut self, k: K) -> Option<&mut Value> {
        k.get_mut(self)
    }
    pub fn take(&mut self) -> Value {
        std::mem::take(self)
    }
}
pub trait Key {
    fn get(self, v: &Value) -> Option<&Value>;
    fn get_mut(self, v: &mut Value) -> Option<&mut Value>;
    fn insert(self, v: &mut Value) -> &mut Value;
}
impl Key for &str {
    fn get(self, v: &Value) -> Option<&Value> {
        v.as_object()?.get(self)
    }
    fn get_mut(self, v: &mut Value) -> Option<&mut Value> {
        v.as_object_mut()?.get_mut(self)
    }
    fn insert(self, v: &mut Value) -> &mut Value {
        if v.is_null() {
            *v = Value::Object(BTreeMap::new());
        }
        v.as_object_mut()
            .expect("record key on non-map")
            .entry(self.into())
            .or_default()
    }
}
impl Key for &String {
    fn get(self, v: &Value) -> Option<&Value> {
        Key::get(self.as_str(), v)
    }
    fn get_mut(self, v: &mut Value) -> Option<&mut Value> {
        self.as_str().get_mut(v)
    }
    fn insert(self, v: &mut Value) -> &mut Value {
        self.as_str().insert(v)
    }
}
impl Key for usize {
    fn get(self, v: &Value) -> Option<&Value> {
        v.as_array()?.get(self)
    }
    fn get_mut(self, v: &mut Value) -> Option<&mut Value> {
        v.as_array_mut()?.get_mut(self)
    }
    fn insert(self, v: &mut Value) -> &mut Value {
        &mut v.as_array_mut().expect("index on non-array")[self]
    }
}
impl Key for String {
    fn get(self, v: &Value) -> Option<&Value> {
        Key::get(self.as_str(), v)
    }
    fn get_mut(self, v: &mut Value) -> Option<&mut Value> {
        Key::get_mut(self.as_str(), v)
    }
    fn insert(self, v: &mut Value) -> &mut Value {
        Key::insert(self.as_str(), v)
    }
}
impl<K: Key> Index<K> for Value {
    type Output = Value;
    fn index(&self, k: K) -> &Value {
        k.get(self).unwrap_or(&Value::Null)
    }
}
impl<K: Key> IndexMut<K> for Value {
    fn index_mut(&mut self, k: K) -> &mut Value {
        k.insert(self)
    }
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl PartialEq<str> for Value {
    fn eq(&self, v: &str) -> bool {
        self.as_str() == Some(v)
    }
}
impl PartialEq<&str> for Value {
    fn eq(&self, v: &&str) -> bool {
        self.as_str() == Some(*v)
    }
}
impl PartialEq<String> for Value {
    fn eq(&self, v: &String) -> bool {
        self.as_str() == Some(v.as_str())
    }
}
impl PartialEq<bool> for Value {
    fn eq(&self, v: &bool) -> bool {
        self.as_bool() == Some(*v)
    }
}
impl PartialEq<Value> for String {
    fn eq(&self, v: &Value) -> bool {
        v == self
    }
}
impl PartialEq<Value> for str {
    fn eq(&self, v: &Value) -> bool {
        v == self
    }
}
impl PartialEq<Value> for &str {
    fn eq(&self, v: &Value) -> bool {
        v == self
    }
}
macro_rules! number_eq { ($($t:ty),*) => { $(impl PartialEq<$t> for Value { fn eq(&self, v: &$t) -> bool { to_value(v).is_ok_and(|x| *self == x) } })* }; }
number_eq!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64);
macro_rules! value_from { ($($t:ty),*) => { $(impl From<$t> for Value {fn from(v:$t)->Self{to_value(v).expect("primitive value")}})* }; }
value_from!(
    u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64, bool, String, &str
);

impl Serialize for Value {
    fn serialize<S: ser::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Null => s.serialize_unit(),
            Self::Bool(v) => s.serialize_bool(*v),
            Self::U64(v) => s.serialize_u64(*v),
            Self::I64(v) => s.serialize_i64(*v),
            Self::F32(v) => s.serialize_f32(*v),
            Self::F64(v) => s.serialize_f64(*v),
            Self::String(v) => s.serialize_str(v),
            Self::Bytes(v) => s.serialize_bytes(v),
            Self::Array(v) => v.serialize(s),
            Self::Object(v) => v.serialize(s),
        }
    }
}
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Value;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("Replica typed value")
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Value, E> {
                Ok(Value::Null)
            }
            fn visit_none<E: de::Error>(self) -> std::result::Result<Value, E> {
                Ok(Value::Null)
            }
            fn visit_some<D: de::Deserializer<'de>>(
                self,
                d: D,
            ) -> std::result::Result<Value, D::Error> {
                Value::deserialize(d)
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Value, E> {
                Ok(Value::Bool(v))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Value, E> {
                Ok(Value::U64(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Value, E> {
                Ok(if v >= 0 {
                    Value::U64(v as u64)
                } else {
                    Value::I64(v)
                })
            }
            fn visit_f32<E: de::Error>(self, v: f32) -> std::result::Result<Value, E> {
                Ok(Value::F32(v))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Value, E> {
                Ok(Value::F64(v))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Value, E> {
                Ok(Value::String(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Value, E> {
                Ok(Value::String(v))
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> std::result::Result<Value, E> {
                Ok(Value::Bytes(v.into()))
            }
            fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> std::result::Result<Value, E> {
                Ok(Value::Bytes(v))
            }
            fn visit_seq<A: de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Value, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element()? {
                    if v.len() >= MAX_ITEMS {
                        return Err(de::Error::custom("item bound"));
                    }
                    v.push(x);
                }
                Ok(Value::Array(v))
            }
            fn visit_map<A: de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Value, A::Error> {
                let mut v = BTreeMap::new();
                while let Some((k, x)) = a.next_entry::<String, Value>()? {
                    if v.len() >= MAX_ITEMS || v.insert(k, x).is_some() {
                        return Err(de::Error::custom("duplicate key/item bound"));
                    }
                }
                Ok(Value::Object(v))
            }
        }
        d.deserialize_any(Visitor)
    }
}

pub fn to_value<T: Serialize>(v: T) -> Result<Value> {
    v.serialize(Serializer)
}
pub fn from_value<T: de::DeserializeOwned>(v: Value) -> Result<T> {
    T::deserialize(v)
}
pub fn describe<T: Serialize + ?Sized>(v: &T) -> Result<String> {
    Ok(format!("{}", to_value(v)?))
}
struct Serializer;
struct Sequence(Vec<Value>, Option<String>);
struct Mapping(BTreeMap<String, Value>, Option<String>, Option<String>);
impl ser::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = Sequence;
    type SerializeTuple = Sequence;
    type SerializeTupleStruct = Sequence;
    type SerializeTupleVariant = Sequence;
    type SerializeMap = Mapping;
    type SerializeStruct = Mapping;
    type SerializeStructVariant = Mapping;
    fn serialize_bool(self, v: bool) -> Result<Value> {
        Ok(Value::Bool(v))
    }
    fn serialize_i8(self, v: i8) -> Result<Value> {
        self.serialize_i64(v.into())
    }
    fn serialize_i16(self, v: i16) -> Result<Value> {
        self.serialize_i64(v.into())
    }
    fn serialize_i32(self, v: i32) -> Result<Value> {
        self.serialize_i64(v.into())
    }
    fn serialize_i64(self, v: i64) -> Result<Value> {
        Ok(if v >= 0 {
            Value::U64(v as u64)
        } else {
            Value::I64(v)
        })
    }
    fn serialize_u8(self, v: u8) -> Result<Value> {
        self.serialize_u64(v.into())
    }
    fn serialize_u16(self, v: u16) -> Result<Value> {
        self.serialize_u64(v.into())
    }
    fn serialize_u32(self, v: u32) -> Result<Value> {
        self.serialize_u64(v.into())
    }
    fn serialize_u64(self, v: u64) -> Result<Value> {
        Ok(Value::U64(v))
    }
    fn serialize_u128(self, v: u128) -> Result<Value> {
        self.serialize_u64(v.try_into().map_err(|_| bad("unsigned integer range"))?)
    }
    fn serialize_i128(self, v: i128) -> Result<Value> {
        if v >= 0 {
            self.serialize_u128(v as u128)
        } else {
            self.serialize_i64(v.try_into().map_err(|_| bad("signed integer range"))?)
        }
    }
    fn serialize_f32(self, v: f32) -> Result<Value> {
        Ok(Value::F32(v))
    }
    fn serialize_f64(self, v: f64) -> Result<Value> {
        Ok(Value::F64(v))
    }
    fn serialize_char(self, v: char) -> Result<Value> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_str(self, v: &str) -> Result<Value> {
        Ok(Value::String(v.into()))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Value> {
        Ok(Value::Bytes(v.into()))
    }
    fn serialize_none(self) -> Result<Value> {
        Ok(Value::Null)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<Value> {
        to_value(v)
    }
    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<Value> {
        Ok(Value::Null)
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, v: &'static str) -> Result<Value> {
        self.serialize_str(v)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        v: &T,
    ) -> Result<Value> {
        to_value(v)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        k: &'static str,
        v: &T,
    ) -> Result<Value> {
        Ok(Value::Object([(k.into(), to_value(v)?)].into()))
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Sequence> {
        Ok(Sequence(Vec::new(), None))
    }
    fn serialize_tuple(self, n: usize) -> Result<Sequence> {
        self.serialize_seq(Some(n))
    }
    fn serialize_tuple_struct(self, _: &'static str, n: usize) -> Result<Sequence> {
        self.serialize_tuple(n)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        v: &'static str,
        _: usize,
    ) -> Result<Sequence> {
        Ok(Sequence(Vec::new(), Some(v.into())))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Mapping> {
        Ok(Mapping(BTreeMap::new(), None, None))
    }
    fn serialize_struct(self, _: &'static str, n: usize) -> Result<Mapping> {
        self.serialize_map(Some(n))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        v: &'static str,
        _: usize,
    ) -> Result<Mapping> {
        Ok(Mapping(BTreeMap::new(), None, Some(v.into())))
    }
    fn is_human_readable(&self) -> bool {
        false
    }
}
impl Sequence {
    fn push<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<()> {
        if self.0.len() >= MAX_ITEMS {
            return Err(bad("sequence bound"));
        }
        self.0.push(to_value(v)?);
        Ok(())
    }
    fn finish(self) -> Result<Value> {
        let v = Value::Array(self.0);
        Ok(match self.1 {
            None => v,
            Some(k) => Value::Object([(k, v)].into()),
        })
    }
}
impl ser::SerializeSeq for Sequence {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<()> {
        self.push(v)
    }
    fn end(self) -> Result<Value> {
        self.finish()
    }
}
impl ser::SerializeTuple for Sequence {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<()> {
        self.push(v)
    }
    fn end(self) -> Result<Value> {
        self.finish()
    }
}
impl ser::SerializeTupleStruct for Sequence {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<()> {
        self.push(v)
    }
    fn end(self) -> Result<Value> {
        self.finish()
    }
}
impl ser::SerializeTupleVariant for Sequence {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<()> {
        self.push(v)
    }
    fn end(self) -> Result<Value> {
        self.finish()
    }
}
impl Mapping {
    fn field<T: Serialize + ?Sized>(&mut self, k: String, v: &T) -> Result<()> {
        if self.0.len() >= MAX_ITEMS || self.0.insert(k, to_value(v)?).is_some() {
            return Err(bad("duplicate map key/item bound"));
        }
        Ok(())
    }
    fn finish(self) -> Result<Value> {
        if self.1.is_some() {
            return Err(bad("map value missing"));
        }
        let v = Value::Object(self.0);
        Ok(match self.2 {
            None => v,
            Some(k) => Value::Object([(k, v)].into()),
        })
    }
}
impl ser::SerializeMap for Mapping {
    type Ok = Value;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, k: &T) -> Result<()> {
        if self.1.is_some() {
            return Err(bad("map value missing"));
        }
        self.1 = Some(match to_value(k)? {
            Value::String(s) => s,
            Value::U64(n) => n.to_string(),
            Value::I64(n) => n.to_string(),
            _ => return Err(bad("map key type")),
        });
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<()> {
        let k = self.1.take().ok_or_else(|| bad("map key missing"))?;
        self.field(k, v)
    }
    fn end(self) -> Result<Value> {
        self.finish()
    }
}
impl ser::SerializeStruct for Mapping {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, k: &'static str, v: &T) -> Result<()> {
        self.field(k.into(), v)
    }
    fn end(self) -> Result<Value> {
        self.finish()
    }
}
impl ser::SerializeStructVariant for Mapping {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, k: &'static str, v: &T) -> Result<()> {
        self.field(k.into(), v)
    }
    fn end(self) -> Result<Value> {
        self.finish()
    }
}

impl<'de> de::IntoDeserializer<'de, Error> for Value {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self {
        self
    }
}
struct MapKey(String);
impl<'de> de::IntoDeserializer<'de, Error> for MapKey {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self {
        self
    }
}
macro_rules! key_number {
    ($method:ident,$ty:ty,$visit:ident) => {
        fn $method<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            let n = self.0.parse::<$ty>().map_err(|_| bad("numeric map key"))?;
            if n.to_string() != self.0 {
                return Err(bad("noncanonical numeric map key"));
            }
            v.$visit(n)
        }
    };
}
impl<'de> de::Deserializer<'de> for MapKey {
    type Error = Error;
    fn deserialize_any<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
        v.visit_string(self.0)
    }
    key_number!(deserialize_u8, u8, visit_u8);
    key_number!(deserialize_u16, u16, visit_u16);
    key_number!(deserialize_u32, u32, visit_u32);
    key_number!(deserialize_u64, u64, visit_u64);
    key_number!(deserialize_i8, i8, visit_i8);
    key_number!(deserialize_i16, i16, visit_i16);
    key_number!(deserialize_i32, i32, visit_i32);
    key_number!(deserialize_i64, i64, visit_i64);
    serde::forward_to_deserialize_any! {bool f32 f64 char str string bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any}
    fn is_human_readable(&self) -> bool {
        false
    }
}
impl<'de> de::Deserializer<'de> for Value {
    type Error = Error;
    fn deserialize_any<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
        match self {
            Self::Null => v.visit_unit(),
            Self::Bool(x) => v.visit_bool(x),
            Self::U64(x) => v.visit_u64(x),
            Self::I64(x) => v.visit_i64(x),
            Self::F32(x) => v.visit_f32(x),
            Self::F64(x) => v.visit_f64(x),
            Self::String(x) => v.visit_string(x),
            Self::Bytes(x) => v.visit_byte_buf(x),
            Self::Array(x) => v.visit_seq(de::value::SeqDeserializer::new(x.into_iter())),
            Self::Object(x) => v.visit_map(de::value::MapDeserializer::new(
                x.into_iter().map(|(k, v)| (MapKey(k), v)),
            )),
        }
    }
    fn deserialize_option<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if self.is_null() {
            v.visit_none()
        } else {
            v.visit_some(self)
        }
    }
    fn deserialize_newtype_struct<V: de::Visitor<'de>>(
        self,
        _: &'static str,
        v: V,
    ) -> Result<V::Value> {
        v.visit_newtype_struct(self)
    }
    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        v: V,
    ) -> Result<V::Value> {
        let (k, x) = match self {
            Self::String(k) => (k, Self::Null),
            Self::Object(mut x) if x.len() == 1 => x.pop_first().unwrap(),
            _ => return Err(bad("enum shape")),
        };
        v.visit_enum(Variant(k, x))
    }
    serde::forward_to_deserialize_any! {bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf unit unit_struct seq tuple tuple_struct map struct identifier ignored_any}
    fn is_human_readable(&self) -> bool {
        false
    }
}
struct Variant(String, Value);
impl<'de> de::EnumAccess<'de> for Variant {
    type Error = Error;
    type Variant = Value;
    fn variant_seed<V: de::DeserializeSeed<'de>>(self, v: V) -> Result<(V::Value, Value)> {
        Ok((
            v.deserialize(de::value::StringDeserializer::<Error>::new(self.0))?,
            self.1,
        ))
    }
}
impl<'de> de::VariantAccess<'de> for Value {
    type Error = Error;
    fn unit_variant(self) -> Result<()> {
        if self.is_null() {
            Ok(())
        } else {
            Err(bad("enum unit"))
        }
    }
    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, v: T) -> Result<T::Value> {
        v.deserialize(self)
    }
    fn tuple_variant<V: de::Visitor<'de>>(self, _: usize, v: V) -> Result<V::Value> {
        de::Deserializer::deserialize_seq(self, v)
    }
    fn struct_variant<V: de::Visitor<'de>>(
        self,
        _: &'static [&'static str],
        v: V,
    ) -> Result<V::Value> {
        de::Deserializer::deserialize_map(self, v)
    }
}

fn encode(v: &Value, b: &mut Vec<u8>, depth: usize, items: &mut usize) -> Result<()> {
    *items += 1;
    if depth > MAX_DEPTH || *items > MAX_ITEMS || b.len() > MAX_FRAME_BYTES {
        return Err(bad("value bounds"));
    }
    match v {
        Value::Null => b.push(0),
        Value::Bool(false) => b.push(1),
        Value::Bool(true) => b.push(2),
        Value::U64(n) => {
            b.push(3);
            put_varint(b, *n)
        }
        Value::I64(n) if *n >= 0 => {
            b.push(3);
            put_varint(b, *n as u64)
        }
        Value::I64(n) => {
            b.push(4);
            put_varint(b, ((*n as u64) << 1) ^ ((*n >> 63) as u64))
        }
        Value::F32(n) => {
            b.push(5);
            b.extend(n.to_bits().to_le_bytes())
        }
        Value::F64(n) => {
            b.push(6);
            b.extend(n.to_bits().to_le_bytes())
        }
        Value::String(s) => {
            b.push(7);
            put_bytes(b, s.as_bytes())
        }
        Value::Bytes(s) => {
            b.push(8);
            put_bytes(b, s)
        }
        Value::Array(a) => {
            b.push(9);
            put_varint(b, a.len() as u64);
            for v in a {
                encode(v, b, depth + 1, items)?
            }
        }
        Value::Object(a) => {
            b.push(10);
            put_varint(b, a.len() as u64);
            for (k, v) in a {
                put_bytes(b, k.as_bytes());
                encode(v, b, depth + 1, items)?
            }
        }
    }
    if b.len() > MAX_FRAME_BYTES {
        return Err(bad("record byte bound"));
    }
    Ok(())
}
fn decode(r: &mut Reader<'_>, depth: usize, items: &mut usize) -> Result<Value> {
    *items += 1;
    if depth > MAX_DEPTH || *items > MAX_ITEMS {
        return Err(bad("value bounds"));
    }
    Ok(match r.byte()? {
        0 => Value::Null,
        1 => Value::Bool(false),
        2 => Value::Bool(true),
        3 => Value::U64(r.var()?),
        4 => {
            let n = r.var()?;
            let n = ((n >> 1) as i64) ^ -((n & 1) as i64);
            if n >= 0 {
                return Err(bad("noncanonical signed value"));
            }
            Value::I64(n)
        }
        5 => Value::F32(f32::from_bits(u32::from_le_bytes(
            r.take(4)?.try_into().unwrap(),
        ))),
        6 => Value::F64(f64::from_bits(u64::from_le_bytes(
            r.take(8)?.try_into().unwrap(),
        ))),
        7 => {
            Value::String(String::from_utf8(r.bytes(MAX_BYTES)?).map_err(|_| bad("strict UTF-8"))?)
        }
        8 => Value::Bytes(r.bytes(MAX_BYTES)?),
        tag @ (9 | 10) => {
            let n = usize::try_from(r.var()?).map_err(|_| bad("count overflow"))?;
            if n > MAX_ITEMS - *items {
                return Err(bad("item bound"));
            }
            if tag == 9 {
                let mut a = Vec::new();
                for _ in 0..n {
                    a.push(decode(r, depth + 1, items)?);
                }
                Value::Array(a)
            } else {
                let mut a = BTreeMap::new();
                for _ in 0..n {
                    let k = String::from_utf8(r.bytes(MAX_BYTES)?).map_err(|_| bad("key UTF-8"))?;
                    if a.last_key_value().is_some_and(|(last, _)| last >= &k) {
                        return Err(bad("duplicate/unordered map key"));
                    }
                    a.insert(k, decode(r, depth + 1, items)?);
                }
                Value::Object(a)
            }
        }
        _ => return Err(bad("unknown value tag")),
    })
}
pub fn to_vec<T: Serialize + ?Sized>(v: &T) -> Result<Vec<u8>> {
    encode_frame(&to_value(v)?)
}
pub fn to_storage_vec<T: Serialize + ?Sized>(v: &T) -> Result<Vec<u8>> {
    to_value(v)?.to_storage_vec()
}
fn encode_frame(v: &Value) -> Result<Vec<u8>> {
    let mut out = vec![0; HEADER];
    encode(v, &mut out, 0, &mut 0)?;
    let len = out.len() - HEADER;
    out[..8].copy_from_slice(MAGIC);
    out[8..10].copy_from_slice(&1u16.to_le_bytes());
    out[12..20].copy_from_slice(&(len as u64).to_le_bytes());
    let digest = Sha256::digest(&out[HEADER..]);
    out[20..HEADER].copy_from_slice(&digest);
    Ok(out)
}
fn frame_size(b: &[u8]) -> Result<usize> {
    if b.len() < HEADER
        || &b[..8] != MAGIC
        || b[8..10] != [1, 0]
        || !matches!(b[10..12], [0, 0] | [1, 0])
    {
        return Err(bad("magic/version/header"));
    }
    let n = u64::from_le_bytes(b[12..20].try_into().unwrap());
    if n > MAX_BYTES as u64 {
        return Err(bad("record size"));
    }
    Ok(HEADER + n as usize)
}
pub fn from_slice<T: de::DeserializeOwned>(b: &[u8]) -> Result<T> {
    from_value(value_from_slice(b)?)
}
/// IPC and standalone tokenizer identity use raw canonical frames, never storage compression.
pub fn from_canonical_slice<T: de::DeserializeOwned>(b: &[u8]) -> Result<T> {
    frame_size(b)?;
    if b[10] != 0 {
        return Err(bad("storage compression is not canonical transport"));
    }
    from_slice(b)
}
pub fn value_from_slice(b: &[u8]) -> Result<Value> {
    decode_frame(b, &mut 0, &mut 0)
}
fn decode_frame(b: &[u8], items: &mut usize, expanded: &mut usize) -> Result<Value> {
    let n = frame_size(b)?;
    if n != b.len() {
        return Err(bad("length/checksum"));
    }
    let packed = &b[HEADER..];
    let decompressed;
    let body = if b[10] == 1 {
        if packed.len() < 8 {
            return Err(bad("compressed length"));
        }
        let raw_len = u64::from_le_bytes(packed[..8].try_into().unwrap());
        if raw_len > MAX_BYTES as u64 || raw_len > (MAX_BYTES - *expanded) as u64 {
            return Err(bad("expanded stream bound"));
        }
        let z = &packed[8..];
        if zstd::zstd_safe::find_frame_compressed_size(z).map_err(|_| bad("compressed frame"))?
            != z.len()
        {
            return Err(bad("compressed trailing bytes"));
        }
        decompressed = zstd::bulk::decompress(z, raw_len as usize)?;
        if decompressed.len() != raw_len as usize {
            return Err(bad("expanded length"));
        }
        decompressed.as_slice()
    } else {
        packed
    };
    *expanded = expanded
        .checked_add(body.len())
        .filter(|n| *n <= MAX_BYTES)
        .ok_or_else(|| bad("expanded stream bound"))?;
    if Sha256::digest(body)[..] != b[20..HEADER] {
        return Err(bad("length/checksum"));
    }
    let mut r = Reader::new(body);
    let v = decode(&mut r, 0, items)?;
    if !r.finished() {
        return Err(bad("trailing payload"));
    }
    Ok(v)
}
pub fn write_record<W: Write, T: Serialize + ?Sized>(w: &mut W, v: &T) -> Result<()> {
    write_value_record(w, &to_value(v)?)
}
pub fn write_value_record<W: Write>(w: &mut W, v: &Value) -> Result<()> {
    w.write_all(&v.to_storage_vec()?)?;
    Ok(())
}
pub fn print_record<T: Serialize + ?Sized>(v: &T) -> Result<()> {
    let mut out = std::io::stdout().lock();
    out.write_all(&to_vec(v)?)?;
    out.flush()?;
    Ok(())
}
pub fn records_from_slice<T: de::DeserializeOwned>(b: &[u8]) -> Result<Vec<T>> {
    records_with(b, from_value)
}
pub fn value_records_from_slice(b: &[u8]) -> Result<Vec<Value>> {
    records_with(b, Ok)
}
fn records_with<T>(mut b: &[u8], convert: impl Fn(Value) -> Result<T>) -> Result<Vec<T>> {
    if b.len() > MAX_FRAME_BYTES {
        return Err(bad("stream bound"));
    }
    let mut out = Vec::new();
    let (mut items, mut expanded) = (0, 0);
    while !b.is_empty() {
        let n = frame_size(b)?;
        if n > b.len() {
            return Err(bad("partial record"));
        }
        out.push(convert(decode_frame(&b[..n], &mut items, &mut expanded)?)?);
        if out.len() > MAX_ITEMS {
            return Err(bad("record count"));
        }
        b = &b[n..];
    }
    Ok(out)
}
pub fn read_records<T: de::DeserializeOwned>(path: &std::path::Path) -> Result<Vec<T>> {
    records_from_slice(&read_record_file(path)?)
}
pub fn read_value_records(path: &std::path::Path) -> Result<Vec<Value>> {
    value_records_from_slice(&read_record_file(path)?)
}
fn read_record_file(path: &std::path::Path) -> Result<Vec<u8>> {
    let f = std::fs::File::open(path)?;
    let mut b = Vec::new();
    f.take(MAX_FRAME_BYTES as u64 + 1).read_to_end(&mut b)?;
    Ok(b)
}

// Construction of native record values for the existing diagnostic/scoring code.
// No text parser, textual wire representation or external codec is involved.
#[macro_export]
macro_rules! record {
    (null)=>{$crate::binary::Value::Null};
    ([$($v:tt)*])=>{{let mut values=Vec::new();$crate::record!(@array values [] $($v)*);$crate::binary::Value::Array(values)}};
    ({$($v:tt)*})=>{{let mut values=::std::collections::BTreeMap::new();$crate::record!(@map values $($v)*);$crate::binary::Value::Object(values)}};
    (@map $m:ident)=>{};
    (@map $m:ident $k:tt : null $(,$($rest:tt)*)?)=>{{$m.insert(($k).to_string(),$crate::record!(null));$crate::record!(@map $m $($($rest)*)?);}};
    (@map $m:ident $k:tt : [$($v:tt)*] $(,$($rest:tt)*)?)=>{{$m.insert(($k).to_string(),$crate::record!([$($v)*]));$crate::record!(@map $m $($($rest)*)?);}};
    (@map $m:ident $k:tt : {$($v:tt)*} $(,$($rest:tt)*)?)=>{{$m.insert(($k).to_string(),$crate::record!({$($v)*}));$crate::record!(@map $m $($($rest)*)?);}};
    (@map $m:ident $k:tt : $v:expr $(,$($rest:tt)*)?)=>{{$m.insert(($k).to_string(),$crate::record!($v));$crate::record!(@map $m $($($rest)*)?);}};
    (@array $a:ident [])=>{};
    (@array $a:ident [] null $(,$($rest:tt)*)?)=>{{$a.push($crate::record!(null));$crate::record!(@array $a [] $($($rest)*)?);}};
    (@array $a:ident [] [$($v:tt)*] $(,$($rest:tt)*)?)=>{{$a.push($crate::record!([$($v)*]));$crate::record!(@array $a [] $($($rest)*)?);}};
    (@array $a:ident [] {$($v:tt)*} $(,$($rest:tt)*)?)=>{{$a.push($crate::record!({$($v)*}));$crate::record!(@array $a [] $($($rest)*)?);}};
    (@array $a:ident [] $v:expr $(,$($rest:tt)*)?)=>{{$a.push($crate::record!($v));$crate::record!(@array $a [] $($($rest)*)?);}};
    ($v:expr)=>{$crate::binary::to_value(&$v).expect("native record value")};
}
pub use crate::record;

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct State {
        step: u64,
        scalar: f64,
        token: u32,
        interrupted: Option<String>,
        status: Status,
    }
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    enum Status {
        Pending,
        Complete { accepted: bool },
        Failed(String),
        Pair(u32, u32),
    }
    #[test]
    fn literal_null_and_typed_numeric_roundtrips() {
        let digest = [
            0x6e, 0x34, 0x0b, 0x9c, 0xff, 0xb3, 0x7a, 0x98, 0x9c, 0xa5, 0x44, 0xe6, 0xbb, 0x78,
            0x0a, 0x2c, 0x78, 0x90, 0x1d, 0x3f, 0xb3, 0x37, 0x38, 0x76, 0x85, 0x11, 0xa3, 0x06,
            0x17, 0xaf, 0xa0, 0x1d,
        ];
        let mut literal = b"R3BIN\0\0\0\x01\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00".to_vec();
        literal.extend(digest);
        literal.push(0);
        assert_eq!(to_vec(&Value::Null).unwrap(), literal);
        assert_eq!(from_slice::<Value>(&literal).unwrap(), Value::Null);
        for status in [
            Status::Pending,
            Status::Complete { accepted: false },
            Status::Failed("취소\0\n".into()),
            Status::Pair(3, u32::MAX),
        ] {
            for bits in [
                0,
                0x8000000000000000,
                0x3fd3333333333334,
                0x0010000000000000,
                0x7fefffffffffffff,
            ] {
                let state = State {
                    step: u64::MAX,
                    scalar: f64::from_bits(bits),
                    token: u32::MAX,
                    interrupted: None,
                    status: status.clone(),
                };
                let raw = to_vec(&state).unwrap();
                let restored: State = from_slice(&raw).unwrap();
                assert_eq!(restored.scalar.to_bits(), bits);
                assert_eq!(restored, state);
                // Also exercise the native dynamic records used by policy/scoring code.
                let restored: State = from_value(to_value(&state).unwrap()).unwrap();
                assert_eq!(restored.scalar.to_bits(), bits);
            }
        }
        for bits in [
            0u64,
            0x8000000000000000,
            0x3fd3333333333334,
            0x0010000000000000,
            0x7ff8000000001234,
        ] {
            let v = f64::from_bits(bits);
            assert_eq!(
                from_slice::<f64>(&to_vec(&v).unwrap()).unwrap().to_bits(),
                bits
            );
        }
        assert_eq!(
            from_slice::<i64>(&to_vec(&i64::MIN).unwrap()).unwrap(),
            i64::MIN
        );
        assert!(from_value::<u32>(Value::U64(u64::MAX)).is_err());
        assert_eq!(
            from_slice::<u128>(&to_vec(&(u64::MAX as u128)).unwrap()).unwrap(),
            u64::MAX as u128
        );
        assert_eq!(
            from_slice::<i128>(&to_vec(&(i64::MIN as i128)).unwrap()).unwrap(),
            i64::MIN as i128
        );
        assert!(to_vec(&u128::MAX).is_err());
        assert!(to_vec(&i128::MIN).is_err());
        assert!(from_value::<State>(record!({"step":0,"scalar":0.,"token":0,"interrupted":null,"status":"Pending","unknown":true})).is_err());
        assert!(
            from_value::<State>(
                record!({"step":0,"scalar":0.,"interrupted":null,"status":"Pending"})
            )
            .is_err()
        );
        let counts: BTreeMap<i64, u64> = [(i64::MIN, u64::MAX), (0, 17), (42, 9)].into();
        assert_eq!(
            from_slice::<BTreeMap<i64, u64>>(&to_vec(&counts).unwrap()).unwrap(),
            counts
        );
        assert!(from_value::<BTreeMap<u32, u64>>(record!({"01":3})).is_err());
    }
    fn frame(body: &[u8]) -> Vec<u8> {
        let mut b = b"R3BIN\0\0\0\x01\x00\x00\x00".to_vec();
        b.extend((body.len() as u64).to_le_bytes());
        b.extend(Sha256::digest(body));
        b.extend(body);
        b
    }
    #[test]
    fn malformed_and_partial_records_are_rejected_without_text_fallback() {
        let valid = to_vec(&record!({"actual":"한글\n\0","raw":[0,255,2],"error":null})).unwrap();
        for n in 0..valid.len() {
            assert!(from_slice::<Value>(&valid[..n]).is_err());
        }
        let mut corrupt = valid.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(from_slice::<Value>(&corrupt).is_err());
        assert!(from_slice::<Value>(b"{\"ready\":true}").is_err());
        for body in [
            vec![255],
            vec![7, 1, 255],
            vec![3, 0x80, 0],
            vec![4, 0],
            vec![10, 2, 1, b'a', 0, 1, b'a', 0],
            vec![0, 0],
        ] {
            assert!(from_slice::<Value>(&frame(&body)).is_err(), "{body:?}");
        }
        let mut nested = Vec::new();
        for _ in 0..66 {
            nested.extend([9, 1]);
        }
        nested.push(0);
        assert!(from_slice::<Value>(&frame(&nested)).is_err());
        let mut huge = valid.clone();
        huge[12..20].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(from_slice::<Value>(&huge).is_err());
        let mut both = valid.clone();
        both.extend(&valid);
        let rows: Vec<Value> = records_from_slice(&both).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(from_slice::<Value>(&both).is_err());
        both.pop();
        assert!(records_from_slice::<Value>(&both).is_err());
        let mut body = Vec::new();
        // Writer and reader count nested nodes alike, even when each container is small.
        assert!(encode(&record!([null]), &mut body, 0, &mut (MAX_ITEMS - 1)).is_err());
    }
    #[test]
    fn storage_compression_preserves_canonical_bits_and_mixed_records() {
        let value = record!({"original":"원문\0\n".repeat(4096),"step":u64::MAX,
            "loss":f64::from_bits(0x3fd3333333333334),"negative_zero":-0.0f64});
        let raw = to_vec(&value).unwrap();
        assert_eq!(value.to_vec().unwrap(), raw);
        let packed = value.to_storage_vec().unwrap();
        assert_eq!(packed[10], 1);
        assert!(packed.len() * 8 < raw.len());
        assert_eq!(&packed[20..HEADER], &raw[20..HEADER]);
        assert_eq!(value_from_slice(&packed).unwrap().to_vec().unwrap(), raw);
        assert_eq!(from_slice::<Value>(&packed).unwrap(), value);
        assert!(from_canonical_slice::<Value>(&packed).is_err());
        assert_eq!(from_canonical_slice::<Value>(&raw).unwrap(), value);
        let mut ipc = (packed.len() as u32).to_le_bytes().to_vec();
        ipc.extend(&packed);
        assert!(crate::model::read_frame::<Value>(&mut ipc.as_slice(), MAX_FRAME_BYTES).is_err());
        let signed = Value::I64(3);
        assert_eq!(signed.to_vec().unwrap(), to_vec(&signed).unwrap());
        let small = record!({"terminal":true,"complete":false});
        assert_eq!(small.to_storage_vec().unwrap(), to_vec(&small).unwrap());
        let mut stream = packed.clone();
        stream.extend(small.to_vec().unwrap());
        stream.extend(&raw);
        let expected = vec![value.clone(), small, value];
        assert_eq!(value_records_from_slice(&stream).unwrap(), expected);
        assert_eq!(records_from_slice::<Value>(&stream).unwrap(), expected);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed.r3rows");
        std::fs::write(&path, &stream).unwrap();
        assert_eq!(read_value_records(&path).unwrap(), expected);
        assert_eq!(read_records::<Value>(&path).unwrap(), expected);
        stream.pop();
        assert!(value_records_from_slice(&stream).is_err());
    }
    #[test]
    fn compressed_frames_reject_corruption_and_share_expansion_and_item_budgets() {
        let v = record!({"text":"repeat".repeat(2048)});
        let raw = v.to_vec().unwrap();
        let packed = v.to_storage_vec().unwrap();
        for at in [8, 10, 12, 20, HEADER, HEADER + 8, packed.len() - 1] {
            let mut broken = packed.clone();
            broken[at] ^= 2;
            assert!(value_from_slice(&broken).is_err(), "offset {at}");
        }
        for n in [0, 8, HEADER, HEADER + 7, packed.len() - 1] {
            assert!(value_from_slice(&packed[..n]).is_err());
        }
        let mut bomb = packed.clone();
        bomb[HEADER..HEADER + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(value_from_slice(&bomb).is_err());
        let mut extra = packed.clone();
        extra.extend_from_slice(&packed[HEADER + 8..]);
        let size = extra.len() - HEADER;
        extra[12..20].copy_from_slice(&(size as u64).to_le_bytes());
        assert!(value_from_slice(&extra).is_err());
        let mut items = MAX_ITEMS - 2;
        let mut expanded = 0;
        decode_frame(&packed, &mut items, &mut expanded).unwrap();
        assert!(decode_frame(&raw, &mut items, &mut expanded).is_err());
        let mut items = 0;
        let mut expanded = MAX_BYTES - (raw.len() - HEADER);
        decode_frame(&packed, &mut items, &mut expanded).unwrap();
        assert!(decode_frame(&packed, &mut items, &mut expanded).is_err());
        // The physical frame budget includes its header; the payload limit remains unchanged.
        let mut header = raw[..HEADER].to_vec();
        header[12..20].copy_from_slice(&(MAX_BYTES as u64).to_le_bytes());
        assert_eq!(frame_size(&header).unwrap(), MAX_FRAME_BYTES);
        let bytes = Value::Bytes((0..65536).map(|i| (i % 251) as u8).collect());
        assert_eq!(bytes.to_storage_vec().unwrap(), bytes.to_vec().unwrap());
    }
}
