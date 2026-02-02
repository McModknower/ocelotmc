use crate::Tag;

pub struct Encoder {
    buffer: String
}

impl Encoder {
    pub fn new() -> Self {
	Self { buffer: String::new()}
    }
    
    pub fn encode_tag(&mut self, tag: &Tag) {
	match tag {
	    Tag::Byte(value) => {
		self.buffer.push_str(&value.to_string());
		self.buffer.push('b');
	    },
	    Tag::Short(value) => {
		self.buffer.push_str(&value.to_string());
		self.buffer.push('s');
	    },
	    Tag::Int(value) => {
		self.buffer.push_str(&value.to_string());
		self.buffer.push('i');
	    },
	    Tag::Long(value) => {
		self.buffer.push_str(&value.to_string());
		self.buffer.push('l');
	    },
	    Tag::Float(value) => {
		self.buffer.push_str(&value.to_string());
		self.buffer.push('f');
	    },
	    Tag::Double(value) => {
		self.buffer.push_str(&value.to_string());
		self.buffer.push('d');
	    },
	    Tag::ByteArray(items) => {
		self.buffer.push_str("[B;");
		let mut first = true;
		for b in items {
		    if first {
			first = false;
		    } else {
			self.buffer.push(',');
		    }
		    self.buffer.push_str(&b.to_string());
		    self.buffer.push('b');
		}
		self.buffer.push(']');
	    },
	    Tag::IntArray(items) => {
		self.buffer.push_str("[I;");
		let mut first = true;
		for b in items {
		    if first {
			first = false;
		    } else {
			self.buffer.push(',');
		    }
		    self.buffer.push_str(&b.to_string());
		}
		self.buffer.push(']');
	    },
	    Tag::LongArray(items) => {
		self.buffer.push_str("[L;");
		let mut first = true;
		for b in items {
		    if first {
			first = false;
		    } else {
			self.buffer.push(',');
		    }
		    self.buffer.push_str(&b.to_string());
		    self.buffer.push('l');
		}
		self.buffer.push(']');
	    }
	    Tag::String(string) => {
		self.encode_string(string);
	    },
	    Tag::List(_tag_type, tags) => {
		self.buffer.push('[');
		let mut first = true;
		for b in tags {
		    if first {
			first = false;
		    } else {
			self.buffer.push(',');
		    }
		    self.encode_tag(b);
		}
		self.buffer.push(']');
	    },
	    Tag::Compound(hash_map) => {
		self.buffer.push('{');
		let mut first = true;
		for (name,value) in hash_map {
		    if first {
			first = false;
		    } else {
			self.buffer.push(',');
		    }
		    self.encode_string(name);
		    self.buffer.push(':');
		    self.encode_tag(value);
		}
		self.buffer.push('}');
	    },
	}
    }

    fn encode_string(&mut self, string: &String) {
        self.buffer.push('"');
        string.chars().flat_map(|c| Self::escape(c)).for_each(|c| self.buffer.push(c));
        self.buffer.push('"');
    }

    fn escape(c: char) -> EscapedIterator {
        match c {
	    '\x08' => EscapedIterator::Escaped('b'),
	    '\x0C' => EscapedIterator::Escaped('f'),
	    '\x0A' => EscapedIterator::Escaped('n'),
	    '\x0D' => EscapedIterator::Escaped('r'),
	    '\x09' => EscapedIterator::Escaped('t'),
	    '\\' => EscapedIterator::Escaped('\\'),
	    '\'' => EscapedIterator::Escaped('\''),
	    '"' => EscapedIterator::Escaped('"'),
	    _ => EscapedIterator::Literal(c),
	}
    }

}


enum EscapedIterator {
    Escaped(char),
    Literal(char),
    Empty
}

impl Iterator for EscapedIterator {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Escaped(c) => {
		*self = Self::Literal(*c);
		Some('\\')
	    },
            Self::Literal(c) => {
		let c = *c;
		*self = Self::Empty;
		Some(c)
	    },
            Self::Empty => None,
        }
    }
}

pub mod decoder {
    use std::num::ParseIntError;

    use crate::Tag;
    use thiserror::Error;

    type Res<'a,T> = Result<(&'a str, T), SnbtError>;

    pub fn decode_tag<'a>(input: &'a str) -> Res<'a, Tag> {
	let input = space(input);
	let oc = input.chars().next();
	if let Some(c) = oc {
	    decode_tag_internal(input, c)
	} else {
	    Err(SnbtError::EmptyInput)
	}
    }

    fn decode_tag_internal<'a>(input: &'a str, c: char) -> Res<'a, Tag> {
        match c {
	    '{' => todo!("Parse Compound"),
	    '[' => todo!("Parse List-like"),
	    '"' | '\'' => todo!("Parse Quoted String"),
	    '0'..='9' | '-' | '.' | '+' => decode_number(input),
	    '_' | 'a'..='z' | 'A'..='Z' => todo!("Parse Unqoted String/bool"),
	    _ => Err(SnbtError::UnknownChar(c))
	}
    }

    fn decode_number<'a>(input: &'a str) -> Res<'a, Tag> {
	debug_assert!(input.chars().next().is_some_and(|c| c.is_digit(10) || c == '-' || c == '+' || c == '.'));
	let mut chars = input.char_indices();
	let mut c = chars.next();
	let negative = if let Some(cc) = c {
	    if cc.1 == '-' {
		c = chars.next();
		true
	    } else if cc.1 == '+' {
		c = chars.next();
		false
	    } else {
		false
	    }
	} else { false };
	let radix = if c.is_some_and(|c| c.1 == '0') {
	    c = chars.next();
	    if c.is_some_and(|c| c.1 == 'b') {
		c = chars.next();
		2
	    } else if c.is_some_and(|c| c.1 == 'x') {
		c = chars.next();
		16
	    } else {
		// no need for backtracking, since a leading 0 does not change the value of the number
		10
	    }
	} else { 10 };
	let num_start = c.map(|cc| cc.0).unwrap_or_else(|| input.len());
	while c.is_some_and(|cc| cc.1.is_digit(radix) || cc.1 == '_') {
	    c = chars.next();
	}
	if c.is_some_and(|cc| cc.1 == '.') {
	    todo!("Handle float");
	}
	let num_end = c.map(|cc| cc.0).unwrap_or_else(|| input.len());


	if num_start == num_end {
	    return Ok((&input[num_start..], Tag::Byte(0)));
	}
	
	// TODO check what kind of num i have
	// TODO handle max negative num
	// TODO handle _
	let raw_num = i64::from_str_radix(&input[num_start..num_end], radix);

	raw_num.map_or_else(
	    |e| Err(e.into()),
	    |num| Ok((&input[num_end..], Tag::Long(
		if negative {-num} else {num}
	    )))
	)
    }

    #[derive(Error, Debug, PartialEq, Eq)]
    pub enum SnbtError {
	#[error("Empty input")]
        EmptyInput,
	#[error("Not an integer {0}")]
	ParseInt(#[from] ParseIntError),
	#[error("Unexpected Character {0}")]
	UnknownChar(char),
    }

    fn space(input: &str) -> &str {
	let opos = input.find(|c: char| !c.is_whitespace());
	if let Some(pos) = opos {
	    &input[pos..]
	} else {
	    ""
	}
    }
    
    // use nom::{IResult, Parser, branch::alt, character::complete::{char, one_of, satisfy, space0}, combinator::{cut, opt, recognize}, error::{Error, ErrorKind}, multi::{many0, many1}, sequence::{preceded, terminated}};

    // pub fn decode_tag(input: &str) -> Result<Tag, nom::Err<nom::error::Error<&str>>> {
    // 	let internal_result = decode_tag_internal(input)?;
    // 	if let ("", result) = internal_result {
    // 	    Ok(result)
    // 	} else {
    // 	    todo!()
    // 	}
    // }

    // fn decode_tag_internal(input: &str) -> IResult<&str, Tag> {
    // 	let (input, _) = space0(input)?;
    // 	let (input, result) = 
    // 	 alt((
    // 	     preceded(satisfy(|c| c == '['), cut(list)),
    // 	     preceded(satisfy(|c| c == '{'), cut(compound)),
    // 	     preceded(satisfy(|c| c == '"'), cut(string)),
    // 	     preceded(satisfy(|c| c.is_digit(10) || c == '.' || c == '-'), cut(number)),
    // 	 )).parse(input)?;
    // 	let (input, _) = space0(input)?;
    // 	Ok((input,result))
    // }

    // fn number(input: &str) -> IResult<&str, Tag> {
    //     alt((
    // 	    decimal,
    // 	    float,
    // 	    hex,
    // 	    bin,
    // 	)).parse(input)
    // }

    // fn decimal(input: &str) -> IResult<&str, Tag> {
    // 	let (input, dec_str) =
    // 	    recognize(
    // 		(
    // 		    opt(char('-')),
    // 		    many1(
    // 			terminated(one_of("0123456789"), many0(char('_')))
    // 		    ),
    // 		)
    // 	    ).parse(input)?;
    // 	if let Ok(dec) = i64::from_str_radix(dec_str, 10) {
    // 	    return Ok((input,Tag::Long(dec)));
    // 	}
    // 	Err(nom::Err::Error(nom::error::Error::new(dec_str, nom::error::ErrorKind::Fail)))
    // }

    // fn float(_input: &str) -> IResult<&str, Tag> {
    // 	todo!()
    // }

    // fn hex(_input: &str) -> IResult<&str, Tag> {
    // 	todo!()
    // }

    // fn bin(_input: &str) -> IResult<&str, Tag> {
    // 	todo!()
    // }

    // fn string(_input: &str) -> IResult<&str, Tag> {
    // 	todo!()
    // }

    // fn compound(_input: &str) -> IResult<&str, Tag> {
    // 	todo!()
    // }

    // fn list(_input: &str) -> IResult<&str, Tag> {
    // 	todo!()
    // }
    
    // fn expect(&mut self, arg: char) -> Option<char> {
    //     if self.peek() == Some(&arg) {
    // 	    self.advance();
    // 	    Some(arg)
    // 	} else {
    // 	    None
    // 	}
    // }

    // fn peek(&self) -> Option<&char> {
    // 	self.buffer.get(self.position)
    // }

    // fn white(&mut self) {
    //     while self.peek().is_some_and(|c| c.is_whitespace()) {
    // 	    self.advance()
    // 	}
    // }

    // fn advance(&mut self) {
    //     self.position+=1;
    // }

    // fn number_end(&self) -> Option<usize> {
    // 	let mut pos = self.position;
    // 	while self.is_part_of_number(self.buffer.get(pos)) {
    // 	    pos+=1;
    // 	}
    // 	if pos == self.position {
    // 	    None
    // 	} else {
    // 	    Some(pos)
    // 	}
    // }

    // fn is_part_of_number(&self, oc: Option<&char>) -> bool {
    //     oc.is_some_and(|c| c.is_digit(10) || *c == '.')
    // }
    #[cfg(test)]
    mod tests {
	use super::*;
	#[test]
	fn skip_space() {
	    assert_eq!(space(""), "");
	    assert_eq!(space("     "), "");
	    assert_eq!(space("abcd"), "abcd");
	    assert_eq!(space("    abcd"), "abcd");
	    assert_eq!(space("abcd    "), "abcd    ");
	    assert_eq!(space("    abcd    "), "abcd    ");
	    assert_eq!(space("    ab  cd    "), "ab  cd    ");
	    assert_eq!(space("ab  cd    "), "ab  cd    ");
	}
	#[test]
	fn decode_number_test() {
	    assert_eq!(decode_number("+"), Ok(("", Tag::Byte(0))));
	    assert_eq!(decode_number("-"), Ok(("", Tag::Byte(0))));
	    assert_eq!(decode_number("+x"), Ok(("x", Tag::Byte(0))));
	    assert_eq!(decode_number("-x"), Ok(("x", Tag::Byte(0))));
	    assert_eq!(decode_number("0y"), Ok(("y", Tag::Byte(0))));
	    assert_eq!(decode_number("+0y"), Ok(("y", Tag::Byte(0))));
	    assert_eq!(decode_number("-0y"), Ok(("y", Tag::Byte(0))));
	    assert_eq!(decode_number("0xy"), Ok(("y", Tag::Byte(0))));
	    assert_eq!(decode_number("+0xy"), Ok(("y", Tag::Byte(0))));
	    assert_eq!(decode_number("-0xy"), Ok(("y", Tag::Byte(0))));
	    assert_eq!(decode_number("0by"), Ok(("y", Tag::Byte(0))));
	    assert_eq!(decode_number("+0by"), Ok(("y", Tag::Byte(0))));
	    assert_eq!(decode_number("-0by"), Ok(("y", Tag::Byte(0))));
	    assert_eq!(decode_number("0000aaa"),Ok(("aaa",Tag::Long(0))));
	    assert_eq!(decode_number("0xAFFExyz"),Ok(("xyz",Tag::Long(0xAFFE))));
	}
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn encode_simple() {
	let data = Tag::Compound(HashMap::from([(
                "name".into(),
                Tag::String("Bananrama".into()),
        )]));

	let mut encoder = Encoder::new();
	encoder.encode_tag(&data);
	assert_eq!(encoder.buffer, "{\"name\":\"Bananrama\"}");
    }
}
