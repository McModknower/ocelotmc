use std::{
    collections::HashMap,
    io::{self, Read, Write},
};

use crate::{NamedTag, Tag, TagType};

pub trait NbtBinaryCodec: Sized {
    fn encode_binary<W: Write>(&self, writer: &mut W) -> io::Result<()>;
    fn decode_binary<R: Read>(reader: &mut R) -> io::Result<Self>;
}

impl NbtBinaryCodec for TagType {
    fn encode_binary<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.as_id().encode_binary(writer)
    }

    fn decode_binary<R: Read>(reader: &mut R) -> io::Result<Self> {
        match Self::from_id(u8::decode_binary(reader)?) {
            Some(res) => Ok(res),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid Tag Type Id",
            )),
        }
    }
}

impl Tag {
    pub fn encode_string<W: Write>(string: &str, writer: &mut W) -> io::Result<()> {
        let data = string.as_bytes();
        writer.write_all(&(data.len() as u16).to_be_bytes())?;
        writer.write_all(data)
    }

    fn encode_binary<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            Self::Byte(data) => data.encode_binary(writer),
            Self::Short(data) => data.encode_binary(writer),
            Self::Int(data) => data.encode_binary(writer),
            Self::Long(data) => data.encode_binary(writer),
            Self::Float(data) => data.encode_binary(writer),
            Self::Double(data) => data.encode_binary(writer),
            Self::ByteArray(items) => items.encode_binary(writer),
            Self::String(string) => string.encode_binary(writer),
            Self::List(tag_type, nameless_tags) => {
                tag_type.as_id().encode_binary(writer)?;
                (nameless_tags.len() as i32).encode_binary(writer)?;
                nameless_tags
                    .iter()
                    .try_for_each(|tag| tag.encode_binary(writer))
            }
            Self::Compound(named_tags) => {
                named_tags.iter().try_for_each(|(name, tag)| {
                    tag.tag_type().encode_binary(writer)?;
                    name.encode_binary(writer)?;
                    tag.encode_binary(writer)
                })?;
                TagType::End.encode_binary(writer)
            }
            Self::IntArray(items) => items.encode_binary(writer),
            Self::LongArray(items) => items.encode_binary(writer),
        }
    }
    fn decode_binary<R: Read>(tag_type: TagType, reader: &mut R) -> io::Result<Self> {
        match tag_type {
            TagType::End => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Trying to deserialize tag of type End",
            )),
            TagType::Byte => Ok(Self::Byte(NbtBinaryCodec::decode_binary(reader)?)),
            TagType::Short => Ok(Self::Short(NbtBinaryCodec::decode_binary(reader)?)),
            TagType::Int => Ok(Self::Int(NbtBinaryCodec::decode_binary(reader)?)),
            TagType::Long => Ok(Self::Long(NbtBinaryCodec::decode_binary(reader)?)),
            TagType::Float => Ok(Self::Float(NbtBinaryCodec::decode_binary(reader)?)),
            TagType::Double => Ok(Self::Double(NbtBinaryCodec::decode_binary(reader)?)),
            TagType::ByteArray => Ok(Self::ByteArray(NbtBinaryCodec::decode_binary(reader)?)),
            TagType::String => Ok(Self::String(NbtBinaryCodec::decode_binary(reader)?)),
            TagType::List => {
                let tag_type = TagType::decode_binary(reader)?;
                let len = i32::decode_binary(reader)? as usize;
                let mut buffer = Vec::with_capacity(len);
                for _ in 0..len {
                    buffer.push(Self::decode_binary(tag_type, reader)?);
                }
                Ok(Self::List(tag_type, buffer))
            }
            TagType::Compound => {
                let mut buffer = HashMap::new();
                let mut tag_type = TagType::decode_binary(reader)?;
                while tag_type != TagType::End {
                    let name = String::decode_binary(reader)?;
                    let tag = Tag::decode_binary(tag_type, reader)?;
                    buffer.insert(name, tag);
                    tag_type = TagType::decode_binary(reader)?;
                }
                Ok(Self::Compound(buffer))
            }
            TagType::IntArray => Ok(Self::IntArray(NbtBinaryCodec::decode_binary(reader)?)),
            TagType::LongArray => Ok(Self::LongArray(NbtBinaryCodec::decode_binary(reader)?)),
        }
    }
}

impl NamedTag {
    fn encode_binary<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.1.tag_type().encode_binary(writer)?;
        self.0.encode_binary(writer)?;
        self.1.encode_binary(writer)
    }

    fn decode_binary<R: Read>(reader: &mut R) -> io::Result<Option<Self>> {
        let tag_type = TagType::decode_binary(reader)?;
        if tag_type == TagType::End {
            Ok(None)
        } else {
            Ok(Some(Self(
                String::decode_binary(reader)?,
                Tag::decode_binary(tag_type, reader)?,
            )))
        }
    }

    fn encode_binary_to_network<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.1.tag_type().encode_binary(writer)?;
        debug_assert_eq!(self.0, "");
        self.1.encode_binary(writer)
    }

    fn decode_binary_from_network<R: Read>(reader: &mut R) -> io::Result<Option<Self>> {
        let tag_type = TagType::decode_binary(reader)?;
        if tag_type == TagType::End {
            Ok(None)
        } else {
            Ok(Some(Self("".into(), Tag::decode_binary(tag_type, reader)?)))
        }
    }
}

macro_rules! int_macro {
    ($type_name:ty) => {
        impl NbtBinaryCodec for $type_name {
            fn encode_binary<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                writer.write_all(&self.to_be_bytes())
            }
            fn decode_binary<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
                let mut buffer = [0u8; size_of::<Self>()];
                reader.read_exact(&mut buffer)?;
                Ok(Self::from_be_bytes(buffer))
            }
        }
    };
}

int_macro!(i8);
int_macro!(u8);
int_macro!(i16);
int_macro!(u16);
int_macro!(i32);
int_macro!(i64);
int_macro!(f32);
int_macro!(f64);

impl NbtBinaryCodec for String {
    fn encode_binary<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let data = self.as_bytes();
        (data.len() as u16).encode_binary(writer)?;
        writer.write_all(data)
    }

    fn decode_binary<R: Read>(reader: &mut R) -> io::Result<Self> {
        let len = u16::decode_binary(reader)? as usize;
        let mut buffer = Vec::new();
        reader.take(len as u64).read_to_end(&mut buffer)?;
        if buffer.len() != len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not enough data for string",
            ));
        }
        Self::from_utf8(buffer).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}

impl<T: NbtBinaryCodec> NbtBinaryCodec for Vec<T> {
    fn encode_binary<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        (self.len() as i32).encode_binary(writer)?;
        self.iter().try_for_each(|data| data.encode_binary(writer))
    }

    fn decode_binary<R: Read>(reader: &mut R) -> io::Result<Self> {
        let len = i32::decode_binary(reader)? as usize;
        let mut buffer = Self::with_capacity(len);
        for _ in 0..len {
            buffer.push(T::decode_binary(reader)?);
        }
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {

    use std::{fs, io::Cursor};

    use super::*;

    #[test]
    fn hello_world_nbt() {
        let data = fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/src/hello_world.nbt")).unwrap();
        let nbt = NamedTag::decode_binary(&mut Cursor::new(data))
            .unwrap()
            .unwrap();
        let expected = NamedTag(
            "hello world".into(),
            Tag::Compound(HashMap::from([(
                "name".into(),
                Tag::String("Bananrama".into()),
            )])),
        );
        assert_eq!(nbt, expected);
    }

    #[test]
    fn hello_world_write() {
        let nbt = NamedTag(
            "hello world".into(),
            Tag::Compound(HashMap::from([(
                "name".into(),
                Tag::String("Bananrama".into()),
            )])),
        );
        let expected =
            fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/src/hello_world.nbt")).unwrap();
        let mut data = Vec::new();
        nbt.encode_binary(&mut data).unwrap();
        assert_eq!(expected, data);
    }

    #[test]
    fn biggest_nbt() {
        let data = fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bigtest.nbt")).unwrap();
        let mut nbt = NamedTag::decode_binary(&mut Cursor::new(data))
            .unwrap()
            .unwrap();

        let mut expected = NamedTag("Level".into(), Tag::Compound(HashMap::from([
	    ("nested compound test".into(), Tag::Compound(HashMap::from([
		("egg".into(), Tag::Compound(HashMap::from([
		    ("name".into(), Tag::String("Eggbert".into())),
		    ("value".into(), Tag::Float(0.5)),
		]))),
		("ham".into(), Tag::Compound(HashMap::from([
		    ("name".into(), Tag::String("Hampus".into())),
		    ("value".into(), Tag::Float(0.75)),
		]))),
	    ]))),
	    ("intTest".into(), Tag::Int(2147483647)),
	    ("byteTest".into(), Tag::Byte(127)),
	    //	    ("stringTest".into(), Tag::String("HELLO WORLD THIS IS A TEST STRING \xc5\xc4\xd6!".into())),
	    ("stringTest".into(), Tag::String("HELLO WORLD THIS IS A TEST STRING ÅÄÖ!".into())),
	    ("listTest (long)".into(), Tag::List(TagType::Long, vec![
		Tag::Long(11),
		Tag::Long(12),
		Tag::Long(13),
		Tag::Long(14),
		Tag::Long(15),
	    ])),
	    ("doubleTest".into(), Tag::Double(0.49312871321823148)),
	    ("floatTest".into(), Tag::Float(0.49823147058486938)),
	    ("longTest".into(), Tag::Long(9223372036854775807)),
	    ("listTest (compound)".into(), Tag::List(TagType::Compound, vec![
		Tag::Compound(HashMap::from([
		    ("created-on".into(), Tag::Long(1264099775885)),
		    ("name".into(), Tag::String("Compound tag #0".into())),
		])),
		Tag::Compound(HashMap::from([
		    ("created-on".into(), Tag::Long(1264099775885)),
		    ("name".into(), Tag::String("Compound tag #1".into())),
		])),
	    ])),
	    ("byteArrayTest (the first 1000 values of (n*n*255+n*7)%100, starting with n=0 (0, 62, 34, 16, 8, ...))".into(), Tag::ByteArray(create_byte_array())),
	    ("shortTest".into(), Tag::Short(32767)),
	])));
        compare_nbt(&mut nbt, &mut expected, &mut vec![]);
        assert_eq!(nbt, expected);
    }

    #[test]
    fn bigtest_nbt_write() {
        let data = fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bigtest.nbt")).unwrap();
        let decoded = NamedTag::decode_binary(&mut Cursor::new(data))
            .unwrap()
            .unwrap();
        let mut encoded = Vec::new();
        decoded.encode_binary(&mut encoded).unwrap();
        let redecoded = NamedTag::decode_binary(&mut Cursor::new(encoded))
            .unwrap()
            .unwrap();
        compare_nbt(&redecoded, &decoded, &mut vec![]);
        assert_eq!(redecoded, decoded);
    }

    #[test]
    fn biggest_nbt_network() {
        let data = fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/bigtest-network.nbt"
        ))
        .unwrap();
        let mut nbt = NamedTag::decode_binary_from_network(&mut Cursor::new(data))
            .unwrap()
            .unwrap();

        let mut expected = NamedTag("".into(), Tag::Compound(HashMap::from([
	    ("nested compound test".into(), Tag::Compound(HashMap::from([
		("egg".into(), Tag::Compound(HashMap::from([
		    ("name".into(), Tag::String("Eggbert".into())),
		    ("value".into(), Tag::Float(0.5)),
		]))),
		("ham".into(), Tag::Compound(HashMap::from([
		    ("name".into(), Tag::String("Hampus".into())),
		    ("value".into(), Tag::Float(0.75)),
		]))),
	    ]))),
	    ("intTest".into(), Tag::Int(2147483647)),
	    ("byteTest".into(), Tag::Byte(127)),
	    //	    ("stringTest".into(), Tag::String("HELLO WORLD THIS IS A TEST STRING \xc5\xc4\xd6!".into())),
	    ("stringTest".into(), Tag::String("HELLO WORLD THIS IS A TEST STRING ÅÄÖ!".into())),
	    ("listTest (long)".into(), Tag::List(TagType::Long, vec![
		Tag::Long(11),
		Tag::Long(12),
		Tag::Long(13),
		Tag::Long(14),
		Tag::Long(15),
	    ])),
	    ("doubleTest".into(), Tag::Double(0.49312871321823148)),
	    ("floatTest".into(), Tag::Float(0.49823147058486938)),
	    ("longTest".into(), Tag::Long(9223372036854775807)),
	    ("listTest (compound)".into(), Tag::List(TagType::Compound, vec![
		Tag::Compound(HashMap::from([
		    ("created-on".into(), Tag::Long(1264099775885)),
		    ("name".into(), Tag::String("Compound tag #0".into())),
		])),
		Tag::Compound(HashMap::from([
		    ("created-on".into(), Tag::Long(1264099775885)),
		    ("name".into(), Tag::String("Compound tag #1".into())),
		])),
	    ])),
	    ("byteArrayTest (the first 1000 values of (n*n*255+n*7)%100, starting with n=0 (0, 62, 34, 16, 8, ...))".into(), Tag::ByteArray(create_byte_array())),
	    ("shortTest".into(), Tag::Short(32767)),
	])));
        compare_nbt(&mut nbt, &mut expected, &mut vec![]);
        assert_eq!(nbt, expected);
    }

    #[test]
    fn bigtest_nbt_network_write() {
        let data = fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/bigtest-network.nbt"
        ))
        .unwrap();
        let decoded = NamedTag::decode_binary_from_network(&mut Cursor::new(data))
            .unwrap()
            .unwrap();
        let mut encoded = Vec::new();
        decoded.encode_binary_to_network(&mut encoded).unwrap();
        let redecoded = NamedTag::decode_binary_from_network(&mut Cursor::new(encoded))
            .unwrap()
            .unwrap();
        compare_nbt(&redecoded, &decoded, &mut vec![]);
        assert_eq!(redecoded, decoded);
    }

    fn compare_nbt(nbt: &NamedTag, expected: &NamedTag, path: &mut Vec<String>) {
        assert_eq!(nbt.0, expected.0, "{:?}", path);
        path.push(nbt.0.clone());
        compare_tag(&nbt.1, &expected.1, path);
        path.pop();
    }

    fn compare_tag(nbt: &Tag, expected: &Tag, path: &mut Vec<String>) {
        match expected {
            Tag::Compound(expected_tags) => match nbt {
                Tag::Compound(nbt_tags) => {
                    assert_eq!(
                        nbt_tags.len(),
                        expected_tags.len(),
                        "Size differs in compound at {:?}",
                        path
                    );
                    for (expected_name, expected_element) in expected_tags {
                        if let Some(nbt_element) = nbt_tags.get(expected_name) {
                            path.push(expected_name.clone());
                            compare_tag(nbt_element, expected_element, path);
                            path.pop();
                        } else {
                            assert!(
                                false,
                                "Expected element with name {:?} at {:?}, other names: {:?}",
                                expected_name,
                                path,
                                nbt_tags.keys()
                            )
                        }
                    }
                }
                _ => assert!(false, "Expected a compound at {:?} but got {:?}", path, nbt),
            },
            Tag::List(expected_tag_type, expected_tags) => match nbt {
                Tag::List(nbt_tag_type, nbt_tags) => {
                    assert_eq!(nbt_tag_type, expected_tag_type, "{:?}", path);
                    assert_eq!(
                        nbt_tags.len(),
                        expected_tags.len(),
                        "Size of List at {:?} differs",
                        path
                    );
                    for i in 0..expected_tags.len() {
                        path.push(i.to_string());
                        compare_tag(&nbt_tags[i], &expected_tags[i], path);
                        path.pop();
                    }
                }
                _ => assert!(false, "Expected a list at {:?} but got {:?}", path, nbt),
            },
            _ => assert_eq!(nbt, expected, "{:?}", path),
        }
    }

    fn create_byte_array() -> Vec<i8> {
        let mut res = Vec::with_capacity(1000);
        for n in 0..1000 {
            res.push(((n * n * 255 + n * 7) % 100) as i8);
        }
        res
    }
}
