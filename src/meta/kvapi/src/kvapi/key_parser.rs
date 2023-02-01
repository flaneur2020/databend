// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::str::Split;

use crate::kvapi::helper::decode_id;
use crate::kvapi::helper::unescape;
use crate::kvapi::KeyError;

/// A helper for parsing a string key into structured key.
pub struct KeyParser<'s> {
    str_key: &'s str,
    /// The index of the next item to return.
    i: usize,
    /// The index of char for the next return.
    index: usize,
    elements: Split<'s, char>,
}

impl<'s> KeyParser<'s> {
    /// Create a new parser, split the str key by delimiter `/`.
    pub fn new(source: &'s str) -> Self {
        Self {
            str_key: source,
            i: 0,
            index: 0,
            elements: source.split('/'),
        }
    }

    /// Similar to `new()` but expect the specified prefix as the first part.
    pub fn new_prefixed(source: &'s str, prefix: &str) -> Result<Self, KeyError> {
        let mut s = Self {
            str_key: source,
            i: 0,
            index: 0,
            elements: source.split('/'),
        };
        s.next_literal(prefix)?;
        Ok(s)
    }

    /// Pop the next element in raw `&str`, without unescaping or decoding.
    ///
    /// If there is no more element, it returns KeyError::WrongNumberOfSegments.
    pub fn next_raw(&mut self) -> Result<&'s str, KeyError> {
        let elt = self.elements.next();

        if let Some(s) = elt {
            self.i += 1;
            self.index += s.len() + 1;
            Ok(s)
        } else {
            Err(KeyError::WrongNumberOfSegments {
                // Expected length
                expect: self.i + 1,
                got: self.str_key.to_string(),
            })
        }
    }

    /// Pop the next element and unescape it.
    pub fn next_str(&mut self) -> Result<String, KeyError> {
        let elt = self.next_raw()?;
        let s = unescape(elt)?;
        Ok(s)
    }

    /// Pop the next element and parse it into a u64.
    pub fn next_u64(&mut self) -> Result<u64, KeyError> {
        let elt = self.next_raw()?;
        decode_id(elt)
    }

    /// Pop the next element that equals `expect`.
    ///
    /// If there is no more element, or the popped element is different, it returns KeyError::WrongNumberOfSegments.
    pub fn next_literal(&mut self, expect: &str) -> Result<(), KeyError> {
        let ith = self.i;
        let elt = self.next_raw()?;

        if elt != expect {
            return Err(KeyError::InvalidSegment {
                i: ith,
                expect: expect.to_string(),
                got: elt.to_string(),
            });
        }

        Ok(())
    }

    /// Return trailing raw string that is not processed.
    pub fn tail_raw(&mut self) -> Result<&str, KeyError> {
        let index = self.index;
        let _ = self.next_raw()?;

        Ok(&self.str_key[index..])
    }

    /// Finish parsing, if there are ore elements left, it returns KeyError::WrongNumberOfSegments
    pub fn done(&mut self) -> Result<(), KeyError> {
        let ith = self.i;
        let elt = self.elements.next();

        if elt.is_some() {
            return Err(KeyError::WrongNumberOfSegments {
                expect: ith,
                got: self.str_key.to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::kvapi::key_parser::KeyParser;

    #[test]
    fn test_key_parser_new_prefixed() -> anyhow::Result<()> {
        {
            let s = "_foo/bar%20-/123";
            let mut kp = KeyParser::new_prefixed(s, "_foo")?;
            assert_eq!(Ok("bar%20-"), kp.next_raw());
        }

        {
            let s = "_foo/bar%20-/123";
            let kp = KeyParser::new_prefixed(s, "bar");
            assert!(kp.is_err());
        }

        Ok(())
    }

    #[test]
    fn test_key_parser_next() -> anyhow::Result<()> {
        let s = "_foo/bar%20-/123";

        let mut kp = KeyParser::new(s);
        assert_eq!(Ok("_foo"), kp.next_raw());
        assert_eq!(Ok("bar%20-"), kp.next_raw());
        assert_eq!(Ok("123"), kp.next_raw());
        assert!(kp.next_raw().is_err());

        Ok(())
    }

    #[test]
    fn test_key_parser_next_str() -> anyhow::Result<()> {
        let s = "_foo/bar%21-/123";

        let mut kp = KeyParser::new(s);
        assert_eq!(Ok("_foo".to_string()), kp.next_str());
        assert_eq!(Ok("bar!-".to_string()), kp.next_str());

        Ok(())
    }

    #[test]
    fn test_key_parser_next_u64() -> anyhow::Result<()> {
        let s = "_foo/bar%20-/123";

        let mut kp = KeyParser::new(s);
        assert!(kp.next_u64().is_err());
        assert!(kp.next_u64().is_err());
        assert_eq!(Ok(123), kp.next_u64());

        Ok(())
    }

    #[test]
    fn test_key_parser_next_literal() -> anyhow::Result<()> {
        let s = "_foo/bar%20-/123";

        let mut kp = KeyParser::new(s);
        assert!(kp.next_literal("_foo").is_ok());
        assert!(kp.next_literal("bar%20-").is_ok());
        assert!(kp.next_literal("foo").is_err());
        assert!(kp.next_literal("123").is_err(), "already consumed");

        Ok(())
    }

    #[test]
    fn test_key_parser_tail() -> anyhow::Result<()> {
        let s = "_foo/bar%20-/123";

        {
            let mut kp = KeyParser::new(s);
            assert_eq!(Ok(s), kp.tail_raw());
        }
        {
            let mut kp = KeyParser::new(s);
            kp.next_raw()?;
            assert_eq!(Ok("bar%20-/123"), kp.tail_raw());
        }
        {
            let mut kp = KeyParser::new(s);
            kp.next_raw()?;
            kp.next_raw()?;
            assert_eq!(Ok("123"), kp.tail_raw());
        }
        {
            let mut kp = KeyParser::new(s);
            kp.next_raw()?;
            kp.next_raw()?;
            kp.next_raw()?;
            assert!(kp.tail_raw().is_err());
        }

        Ok(())
    }

    #[test]
    fn test_key_parser_done() -> anyhow::Result<()> {
        let s = "_foo/bar%20-/123";

        let mut kp = KeyParser::new(s);
        assert!(kp.done().is_err());
        assert!(kp.next_literal("bar%20-").is_ok());
        assert!(kp.next_literal("123").is_ok());
        assert!(kp.done().is_ok());

        Ok(())
    }
}
