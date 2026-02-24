use crate::Context;
use crate::datatype::relax::normalize_whitespace;
use lazy_static::lazy_static;
use relaxng_syntax::types;
use relaxng_syntax::types::DatatypeName;
use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

pub const NAMESPACE_URI: &str = "http://www.w3.org/2001/XMLSchema-datatypes";

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum XsdDatatypeValues {
    String(String),
    Token(String),
    QName(QNameVal),
}

impl super::Datatype for XsdDatatypeValues {
    fn is_valid(&self, value: &str) -> bool {
        match self {
            XsdDatatypeValues::String(s) => s == value,
            XsdDatatypeValues::Token(s) => s == &normalize_whitespace(value),
            // QName validation requires namespace context; is_valid_with_ns should be used instead.
            // Without namespace context we cannot resolve prefixes, so we return false.
            XsdDatatypeValues::QName(_) => false,
        }
    }
}

impl XsdDatatypeValues {
    pub fn is_valid_with_ns(&self, value: &str, ns: &dyn super::Namespaces) -> bool {
        use super::Datatype as _;
        match self {
            XsdDatatypeValues::QName(v) => QNameVal::from_val_with_dyn_ns(value, ns)
                .map(|val| &val == v)
                .unwrap_or(false),
            _ => self.is_valid(value),
        }
    }
}

lazy_static! {
    static ref LANG_RE: regex::Regex = regex::Regex::new(r"^[a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*$").unwrap();
    static ref DATETIME_RE: regex::Regex = regex::Regex::new(r"^-?\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:Z(?:(?:\+|-)\d{2}:\d{2})?)?$").unwrap();
    static ref DURATION_RE: regex::Regex = regex::Regex::new(r"^P(([0-9]+([.,][0-9]*)?Y)?([0-9]+([.,][0-9]*)?M)?([0-9]+([.,][0-9]*)?D)?T?([0-9]+([.,][0-9]*)?H)?([0-9]+([.,][0-9]*)?M)?([0-9]+([.,][0-9]*)?S)?)|\d{4}-?(0[1-9]|11|12)-?(?:[0-2]\d|30|31)T((?:[0-1][0-9]|[2][0-3]):?(?:[0-5][0-9]):?(?:[0-5][0-9]|60)|2400|24:00)$").unwrap();
    static ref TIME_RE: regex::Regex = regex::Regex::new(r"^\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?$").unwrap();
    static ref GYEAR_RE: regex::Regex = regex::Regex::new(r"^-?\d{4,}(Z|[+-]\d{2}:\d{2})?$").unwrap();
    static ref GYEARMONTH_RE: regex::Regex = regex::Regex::new(r"^-?\d{4,}-\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
    static ref GMONTH_RE: regex::Regex = regex::Regex::new(r"^--\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
    static ref GMONTHDAY_RE: regex::Regex = regex::Regex::new(r"^--\d{2}-\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
    static ref GDAY_RE: regex::Regex = regex::Regex::new(r"^---\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
    static ref BASE64_RE: regex::Regex = regex::Regex::new(r"^[A-Za-z0-9+/\s]*={0,2}$").unwrap();
    static ref HEXBINARY_RE: regex::Regex = regex::Regex::new(r"^([0-9A-Fa-f]{2})*$").unwrap();
}

// TODO: actually apply all required facets to each datatype
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum XsdDatatypes {
    NormalizedString(StringFacets),
    String(StringFacets),
    Short(MinMaxFacet<i16>, Option<PatternFacet>),
    UnsignedShort(MinMaxFacet<u16>, Option<PatternFacet>),
    Long(MinMaxFacet<i64>, Option<PatternFacet>),
    Int(MinMaxFacet<i32>, Option<PatternFacet>),
    Integer(MinMaxFacet<num_bigint::BigInt>, Option<PatternFacet>),
    PositiveInteger(MinMaxFacet<num_bigint::BigUint>, Option<PatternFacet>),
    UnsignedInt(MinMaxFacet<u32>, Option<PatternFacet>),
    UnsignedLong(MinMaxFacet<u64>, Option<PatternFacet>),
    Decimal {
        min_max: MinMaxFacet<bigdecimal::BigDecimal>,
        pattern: Option<PatternFacet>,
        fraction_digits: Option<u16>,
        total_digits: Option<u16>,
    },
    Double(Option<PatternFacet>),
    NmTokens(LengthFacet),
    NmToken(LengthFacet),
    NcName(LengthFacet),
    Token(LengthFacet),
    Duration(Option<PatternFacet>),
    Date(Option<PatternFacet>),
    Datetime(Option<PatternFacet>),
    AnyURI(Option<PatternFacet>),
    Language(Option<PatternFacet>),
    Boolean(Option<PatternFacet>),
    Id(Option<PatternFacet>),
    IdRef(Option<PatternFacet>),
    // Previously unsupported types (Bug #4)
    Float(Option<PatternFacet>),
    NonNegativeInteger(MinMaxFacet<num_bigint::BigUint>, Option<PatternFacet>),
    NegativeInteger(MinMaxFacet<num_bigint::BigInt>, Option<PatternFacet>),
    NonPositiveInteger(MinMaxFacet<num_bigint::BigInt>, Option<PatternFacet>),
    Byte(MinMaxFacet<i8>, Option<PatternFacet>),
    UnsignedByte(MinMaxFacet<u8>, Option<PatternFacet>),
    Base64Binary(LengthFacet),
    HexBinary(LengthFacet),
    GYear(Option<PatternFacet>),
    GYearMonth(Option<PatternFacet>),
    GMonth(Option<PatternFacet>),
    GMonthDay(Option<PatternFacet>),
    GDay(Option<PatternFacet>),
    Name(LengthFacet),
    QNameData,
    Entity(LengthFacet),
    Time(Option<PatternFacet>),
}
impl super::Datatype for XsdDatatypes {
    fn is_valid(&self, value: &str) -> bool {
        match self {
            XsdDatatypes::NormalizedString(str_facets) => {
                let normal_val = super::relax::normalize_whitespace(value);
                str_facets.is_valid(&normal_val)
            }
            XsdDatatypes::String(str_facets) => str_facets.is_valid(value),
            XsdDatatypes::Short(min_max, patt) => {
                i16::from_str(value)
                    .ok()
                    .is_some_and(|v| min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::UnsignedShort(min_max, patt) => {
                u16::from_str(value)
                    .ok()
                    .is_some_and(|v| min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Long(min_max, patt) => {
                i64::from_str(value)
                    .ok()
                    .is_some_and(|v| min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Int(min_max, patt) => {
                i32::from_str(value)
                    .ok()
                    .is_some_and(|v| min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Integer(min_max, patt) => {
                num_bigint::BigInt::from_str(value)
                    .ok()
                    .is_some_and(|v| min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::PositiveInteger(min_max, patt) => {
                let one = num_bigint::BigUint::from(1u32);
                num_bigint::BigUint::from_str(value)
                    .ok()
                    .is_some_and(|v| v >= one && min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Decimal {
                min_max,
                pattern: pat,
                fraction_digits: _,
                total_digits: _,
            } => {
                bigdecimal::BigDecimal::from_str(value)
                    .ok()
                    .is_some_and(|v| min_max.is_valid(&v))
                    && pat.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::NmTokens(len) => {
                is_valid_nmtokens(value) && {
                    // length facets on NMTOKENS count the number of tokens
                    let token_count = value.split_ascii_whitespace().count();
                    match len {
                        LengthFacet::Unbounded => true,
                        LengthFacet::MinLength(min) => token_count >= *min,
                        LengthFacet::MaxLength(max) => token_count <= *max,
                        LengthFacet::MinMaxLength(min, max) => {
                            token_count >= *min && token_count <= *max
                        }
                        LengthFacet::Length(l) => token_count == *l,
                    }
                }
            }
            XsdDatatypes::NmToken(len) => is_valid_nmtoken(value) && len.is_valid(value),
            XsdDatatypes::NcName(len) => len.is_valid(value) && is_valid_ncname(value),
            XsdDatatypes::Token(len) => {
                // token: whitespace-collapsed string (no leading/trailing space,
                // no consecutive internal spaces)
                normalize_whitespace(value) == value && len.is_valid(value)
            }
            XsdDatatypes::Duration(patt) => {
                DURATION_RE.is_match(value)
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Date(patt) => {
                chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Datetime(patt) => {
                DATETIME_RE.is_match(value)
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Double(patt) => {
                value.parse::<f64>().is_ok()
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::AnyURI(patt) => {
                // XSD anyURI accepts any string (Jing / XSD 1.0 practice).
                // Whitespace collapsing is applied by the validator before this point.
                patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Language(patt) => {
                LANG_RE.is_match(value)
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Boolean(patt) => {
                (value == "true" || value == "false" || value == "1" || value == "0")
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::UnsignedInt(min_max, patt) => {
                u32::from_str(value)
                    .ok()
                    .is_some_and(|v| min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::UnsignedLong(min_max, patt) => {
                u64::from_str(value)
                    .ok()
                    .is_some_and(|v| min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Id(patt) => patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true),
            XsdDatatypes::IdRef(patt) => patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true),
            XsdDatatypes::Float(patt) => {
                value.parse::<f32>().is_ok()
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::NonNegativeInteger(min_max, patt) => {
                num_bigint::BigUint::from_str(value)
                    .ok()
                    .is_some_and(|v| min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::NegativeInteger(min_max, patt) => {
                let minus_one = num_bigint::BigInt::from(-1i32);
                num_bigint::BigInt::from_str(value)
                    .ok()
                    .is_some_and(|v| v <= minus_one && min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::NonPositiveInteger(min_max, patt) => {
                let zero = num_bigint::BigInt::from(0i32);
                num_bigint::BigInt::from_str(value)
                    .ok()
                    .is_some_and(|v| v <= zero && min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Byte(min_max, patt) => {
                i8::from_str(value)
                    .ok()
                    .is_some_and(|v| min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::UnsignedByte(min_max, patt) => {
                u8::from_str(value)
                    .ok()
                    .is_some_and(|v| min_max.is_valid(&v))
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Base64Binary(len) => {
                BASE64_RE.is_match(value) && {
                    // length facet counts decoded octets
                    let stripped: String = value.chars().filter(|c| !c.is_ascii_whitespace()).collect();
                    // base64 string length in chars / 4 * 3 (minus padding)
                    let char_len = stripped.len();
                    let pad = stripped.chars().rev().take_while(|&c| c == '=').count();
                    let decoded_len = if char_len == 0 { 0 } else { char_len * 3 / 4 - pad };
                    match len {
                        LengthFacet::Unbounded => true,
                        LengthFacet::MinLength(min) => decoded_len >= *min,
                        LengthFacet::MaxLength(max) => decoded_len <= *max,
                        LengthFacet::MinMaxLength(min, max) => decoded_len >= *min && decoded_len <= *max,
                        LengthFacet::Length(l) => decoded_len == *l,
                    }
                }
            }
            XsdDatatypes::HexBinary(len) => {
                HEXBINARY_RE.is_match(value) && {
                    // length facet counts octets (hex chars / 2)
                    let octet_len = value.len() / 2;
                    match len {
                        LengthFacet::Unbounded => true,
                        LengthFacet::MinLength(min) => octet_len >= *min,
                        LengthFacet::MaxLength(max) => octet_len <= *max,
                        LengthFacet::MinMaxLength(min, max) => octet_len >= *min && octet_len <= *max,
                        LengthFacet::Length(l) => octet_len == *l,
                    }
                }
            }
            XsdDatatypes::GYear(patt) => {
                GYEAR_RE.is_match(value)
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::GYearMonth(patt) => {
                GYEARMONTH_RE.is_match(value)
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::GMonth(patt) => {
                GMONTH_RE.is_match(value)
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::GMonthDay(patt) => {
                GMONTHDAY_RE.is_match(value)
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::GDay(patt) => {
                GDAY_RE.is_match(value)
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
            XsdDatatypes::Name(len) => is_valid_name(value) && len.is_valid(value),
            XsdDatatypes::QNameData => is_valid_qname_syntax(value),
            XsdDatatypes::Entity(len) => is_valid_ncname(value) && len.is_valid(value),
            XsdDatatypes::Time(patt) => {
                TIME_RE.is_match(value)
                    && patt.as_ref().map(|p| p.1.is_match(value)).unwrap_or(true)
            }
        }
    }
}

fn is_valid_ncname(text: &str) -> bool {
    match relaxng_syntax::compact::nc_name(relaxng_syntax::compact::Span::new(text)) {
        Ok((rest, _name)) => rest.fragment().is_empty(),
        Err(_) => false,
    }
}

/// XML 1.0 NameChar: like NCNameChar but also allows ':'
fn is_name_char(c: char) -> bool {
    c == ':' || relaxng_syntax::ncname::is_nc_name_char(c)
}

/// Validate an NMTOKEN (one or more XML NameChars)
fn is_valid_nmtoken(text: &str) -> bool {
    !text.is_empty() && text.chars().all(is_name_char)
}

/// Validate NMTOKENS (whitespace-separated list of one or more NMTOKENs)
fn is_valid_nmtokens(text: &str) -> bool {
    let tokens: Vec<&str> = text.split_ascii_whitespace().collect();
    !tokens.is_empty() && tokens.iter().all(|t| is_valid_nmtoken(t))
}

/// XML 1.0 NameStartChar: like NCNameStartChar but also allows ':'
fn is_name_start_char(c: char) -> bool {
    c == ':' || relaxng_syntax::ncname::is_nc_name_start_char(c)
}

/// Validate an XML Name (NameStartChar followed by NameChar*)
fn is_valid_name(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        None => false,
        Some(first) => is_name_start_char(first) && chars.all(is_name_char),
    }
}

/// Validate QName syntax: NCName or NCName:NCName (no namespace resolution)
fn is_valid_qname_syntax(text: &str) -> bool {
    if let Some(pos) = text.find(':') {
        let prefix = &text[..pos];
        let local = &text[pos + 1..];
        is_valid_ncname(prefix) && is_valid_ncname(local)
    } else {
        is_valid_ncname(text)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct StringFacets {
    len: LengthFacet,
    pattern: Option<PatternFacet>,
}
impl StringFacets {
    fn is_valid(&self, value: &str) -> bool {
        self.len.is_valid(value)
            && if let Some(ref pat) = self.pattern {
                pat.is_valid(value)
            } else {
                true
            }
    }

    pub fn bounded(&self) -> bool {
        !matches!(self.len, LengthFacet::Unbounded)
    }

    pub fn min_len(&self) -> Option<usize> {
        match self.len {
            LengthFacet::Unbounded => None,
            LengthFacet::MinLength(min) => Some(min),
            LengthFacet::MaxLength(_) => None,
            LengthFacet::MinMaxLength(min, _) => Some(min),
            LengthFacet::Length(len) => Some(len),
        }
    }

    pub fn max_len(&self) -> Option<usize> {
        match self.len {
            LengthFacet::Unbounded => None,
            LengthFacet::MinLength(_) => None,
            LengthFacet::MaxLength(max) => Some(max),
            LengthFacet::MinMaxLength(_, max) => Some(max),
            LengthFacet::Length(len) => Some(len),
        }
    }

    pub fn regex(&self) -> Option<&regex::Regex> {
        self.pattern.as_ref().map(|pat| &pat.1)
    }
}

#[derive(Debug)]
pub enum XsdDatatypeError {
    Facet {
        type_name: &'static str,
        facet: FacetError,
    },
    UnsupportedDatatype {
        span: codemap::Span,
        name: String,
    },
    InvalidValueOfType {
        span: codemap::Span,
        type_name: &'static str,
    },
}
#[derive(Debug)]
pub enum FacetError {
    ConflictingFacet(&'static str),
    InvalidInt(codemap::Span, String),
    InvalidFloat(codemap::Span, String),
    InvalidPattern(codemap::Span, regex::Error),
    InvalidFacet(codemap::Span, String),
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum LengthFacet {
    Unbounded,
    MinLength(usize),
    MaxLength(usize),
    MinMaxLength(usize, usize),
    Length(usize),
}
impl LengthFacet {
    fn is_valid(&self, value: &str) -> bool {
        let actual = value.chars().count();
        match self {
            LengthFacet::Unbounded => true,
            LengthFacet::MinLength(min) => *min <= actual,
            LengthFacet::MaxLength(max) => actual <= *max,
            LengthFacet::MinMaxLength(min, max) => *min <= actual && actual <= *max,
            LengthFacet::Length(len) => actual == *len,
        }
    }

    fn merge(&mut self, other: LengthFacet) -> Result<(), FacetError> {
        *self = match self {
            LengthFacet::Unbounded => other,
            LengthFacet::MinLength(min) => match other {
                LengthFacet::Unbounded | LengthFacet::MinMaxLength(_, _) => unreachable!(),
                LengthFacet::MinLength(_min) => {
                    return Err(FacetError::ConflictingFacet("minLength"));
                }
                LengthFacet::MaxLength(max) => {
                    if *min > max {
                        return Err(FacetError::ConflictingFacet(
                            "minLength greater than maxLength",
                        ));
                    }
                    LengthFacet::MinMaxLength(*min, max)
                }
                LengthFacet::Length(_) => return Err(FacetError::ConflictingFacet("length")),
            },
            LengthFacet::MaxLength(_) => {
                unimplemented!()
            }
            LengthFacet::MinMaxLength(_, _) => {
                unimplemented!()
            }
            LengthFacet::Length(_) => {
                unimplemented!()
            }
        };
        Ok(())
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum Min<T: PartialOrd> {
    Unbounded,
    Inclusive(T),
    Exclusive(T),
}
impl<T: PartialOrd> Min<T> {
    fn is_valid(&self, v: &T) -> bool {
        match self {
            Min::Unbounded => true,
            Min::Inclusive(min) => min <= v,
            Min::Exclusive(min) => min < v,
        }
    }
}
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum Max<T: PartialOrd> {
    Unbounded,
    Inclusive(T),
    Exclusive(T),
}
impl<T: PartialOrd> Max<T> {
    fn is_valid(&self, v: &T) -> bool {
        match self {
            Max::Unbounded => true,
            Max::Inclusive(max) => v <= max,
            Max::Exclusive(max) => v < max,
        }
    }
}
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct MinMaxFacet<T: PartialOrd> {
    min: Min<T>,
    max: Max<T>,
}
impl<T: PartialOrd> Default for MinMaxFacet<T> {
    fn default() -> Self {
        MinMaxFacet {
            min: Min::Unbounded,
            max: Max::Unbounded,
        }
    }
}

impl<T> MinMaxFacet<T>
where
    T: PartialOrd + Copy + std::ops::Add<Output = T> + From<u8>,
{
    // return the min inclusive value
    pub fn min(&self) -> Option<T> {
        match &self.min {
            Min::Unbounded => None,
            Min::Inclusive(min) => Some(*min),
            Min::Exclusive(min) => Some(*min + T::from(1)),
        }
    }
}

impl<T> MinMaxFacet<T>
where
    T: PartialOrd + Copy + std::ops::Sub<Output = T> + From<u8>,
{
    // return the max inclusive value
    pub fn max(&self) -> Option<T> {
        match &self.max {
            Max::Unbounded => None,
            Max::Inclusive(max) => Some(*max),
            Max::Exclusive(max) => Some(*max - T::from(1)),
        }
    }
}

impl<T> MinMaxFacet<T>
where
    T: PartialOrd + Clone + std::ops::Add<Output = T> + From<u8>,
{
    /// Return the min inclusive value (for types that implement Clone but not Copy)
    pub fn min_cloned(&self) -> Option<T> {
        match &self.min {
            Min::Unbounded => None,
            Min::Inclusive(min) => Some(min.clone()),
            Min::Exclusive(min) => Some(min.clone() + T::from(1)),
        }
    }
}

impl<T> MinMaxFacet<T>
where
    T: PartialOrd + Clone + std::ops::Sub<Output = T> + From<u8>,
{
    /// Return the max inclusive value (for types that implement Clone but not Copy)
    pub fn max_cloned(&self) -> Option<T> {
        match &self.max {
            Max::Unbounded => None,
            Max::Inclusive(max) => Some(max.clone()),
            Max::Exclusive(max) => Some(max.clone() - T::from(1)),
        }
    }
}

impl<T> MinMaxFacet<T>
where
    T: PartialOrd,
{
    pub fn bounded(&self) -> bool {
        !matches!((&self.min, &self.max), (Min::Unbounded, Max::Unbounded))
    }

    fn min_inclusive(&mut self, val: T) -> Result<(), FacetError> {
        match &self.max {
            Max::Unbounded => {}
            Max::Inclusive(max) => {
                if val > *max {
                    return Err(FacetError::ConflictingFacet(
                        "minInclusive conflicts with maxInclusive",
                    ));
                }
            }
            Max::Exclusive(max) => {
                if val >= *max {
                    return Err(FacetError::ConflictingFacet(
                        "minInclusive conflicts with maxExclusive",
                    ));
                }
            }
        }
        self.min = match self.min {
            Min::Unbounded => Min::Inclusive(val),
            Min::Inclusive(_) => unreachable!(),
            Min::Exclusive(_) => {
                return Err(FacetError::ConflictingFacet(
                    "minInclusive conflicts with minExclusive",
                ));
            }
        };
        Ok(())
    }
    fn min_exclusive(&mut self, val: T) -> Result<(), FacetError> {
        match &self.max {
            Max::Unbounded => {}
            Max::Inclusive(max) => {
                if val > *max {
                    return Err(FacetError::ConflictingFacet(
                        "minInclusive conflicts with maxInclusive",
                    ));
                }
            }
            Max::Exclusive(max) => {
                if val >= *max {
                    return Err(FacetError::ConflictingFacet(
                        "minInclusive conflicts with maxExclusive",
                    ));
                }
            }
        }
        self.min = match self.min {
            Min::Unbounded => Min::Exclusive(val),
            Min::Inclusive(_) => {
                return Err(FacetError::ConflictingFacet(
                    "minExclusive conflicts with minInclusive",
                ));
            }
            Min::Exclusive(_) => unreachable!(),
        };
        Ok(())
    }
    fn max_inclusive(&mut self, val: T) -> Result<(), FacetError> {
        match &self.min {
            Min::Unbounded => {}
            Min::Inclusive(min) => {
                if *min > val {
                    return Err(FacetError::ConflictingFacet(
                        "minInclusive conflicts with maxInclusive",
                    ));
                }
            }
            Min::Exclusive(min) => {
                if *min >= val {
                    return Err(FacetError::ConflictingFacet(
                        "minInclusive conflicts with maxExclusive",
                    ));
                }
            }
        }
        self.max = match self.max {
            Max::Unbounded => Max::Inclusive(val),
            Max::Inclusive(_) => unreachable!(),
            Max::Exclusive(_) => {
                return Err(FacetError::ConflictingFacet(
                    "maxInclusive conflicts with maxExclusive",
                ));
            }
        };
        Ok(())
    }
    fn max_exclusive(&mut self, val: T) -> Result<(), FacetError> {
        match &self.min {
            Min::Unbounded => {}
            Min::Inclusive(min) => {
                if *min > val {
                    return Err(FacetError::ConflictingFacet(
                        "minInclusive conflicts with maxInclusive",
                    ));
                }
            }
            Min::Exclusive(min) => {
                if *min >= val {
                    return Err(FacetError::ConflictingFacet(
                        "minInclusive conflicts with maxExclusive",
                    ));
                }
            }
        }
        self.max = match self.max {
            Max::Unbounded => Max::Exclusive(val),
            Max::Inclusive(_) => {
                return Err(FacetError::ConflictingFacet(
                    "maxExclusive conflicts with maxInclusive",
                ));
            }
            Max::Exclusive(_) => unreachable!(),
        };
        Ok(())
    }

    fn is_valid(&self, v: &T) -> bool {
        self.min.is_valid(v) && self.max.is_valid(v)
    }
}

#[derive(Clone)]
pub struct PatternFacet(String, regex::Regex);
impl PartialEq for PatternFacet {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for PatternFacet {}
impl std::hash::Hash for PatternFacet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}
impl fmt::Debug for PatternFacet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_tuple("PatternFacet").field(&self.0).finish()
    }
}
impl PatternFacet {
    fn is_valid(&self, value: &str) -> bool {
        self.1.is_match(value)
    }
}

#[derive(Default)]
pub struct Compiler;
impl super::DatatypeCompiler for Compiler {
    type DT = XsdDatatypes;
    type DTValue = XsdDatatypeValues;
    type Error = XsdDatatypeError;

    fn datatype_value(
        &self,
        ctx: &Context,
        datatype_name: &types::DatatypeName,
        value: &str,
        ns: &[(String, String)],
    ) -> Result<Self::DTValue, Self::Error> {
        match datatype_name {
            DatatypeName::CName(types::QName(_namespace_uri, name)) => {
                self.compile_value(ctx, &name.0, &name.1, value, ns)
            }
            DatatypeName::NamespacedName(_) => {
                unimplemented!()
            }
            _ => panic!("Unexpected {datatype_name:?}"),
        }
    }

    fn datatype_name(
        &self,
        ctx: &Context,
        datatype_name: &types::DatatypeName,
        params: &[types::Param],
    ) -> Result<Self::DT, Self::Error> {
        match datatype_name {
            types::DatatypeName::CName(types::QName(_namespace_uri, name)) => {
                self.compile(ctx, &name.0, &name.1, params)
            }
            _ => panic!("Unexpected {datatype_name:?}"),
        }
    }
}

impl Compiler {
    fn compile(
        &self,
        ctx: &Context,
        span: &types::Span,
        name: &str,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, XsdDatatypeError> {
        match name {
            "normalizedString" => {
                self.normalized_string(ctx, params)
                    .map_err(|facet| XsdDatatypeError::Facet {
                        type_name: "normalizedString",
                        facet,
                    })
            }
            "string" => self
                .string(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "string",
                    facet,
                }),
            "short" => self
                .short(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "short",
                    facet,
                }),
            "unsignedShort" => {
                self.unsigned_short(ctx, params)
                    .map_err(|facet| XsdDatatypeError::Facet {
                        type_name: "unsignedShort",
                        facet,
                    })
            }
            "long" => self
                .long(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "long",
                    facet,
                }),
            "int" => self
                .int(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "int",
                    facet,
                }),
            "integer" => self
                .integer(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "integer",
                    facet,
                }),
            "positiveInteger" => {
                self.positive_integer(ctx, params)
                    .map_err(|facet| XsdDatatypeError::Facet {
                        type_name: "positiveInteger",
                        facet,
                    })
            }
            "decimal" => self
                .decimal(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "decimal",
                    facet,
                }),
            "double" => self
                .double(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "double",
                    facet,
                }),
            "NMTOKENS" => self
                .nmtokens(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "NMTOKENS",
                    facet,
                }),
            "NMTOKEN" => self
                .nmtoken(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "NMTOKEN",
                    facet,
                }),
            "NCName" => self
                .ncname(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "NCName",
                    facet,
                }),
            "token" => self
                .token(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "token",
                    facet,
                }),
            "duration" => self
                .duration(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "duration",
                    facet,
                }),
            "date" => self
                .date(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "date",
                    facet,
                }),
            "dateTime" => self
                .datetime(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "dateTime",
                    facet,
                }),
            "anyURI" => self
                .any_uri(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "anyURI",
                    facet,
                }),
            "language" => self
                .language(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "language",
                    facet,
                }),
            "boolean" => self
                .boolean(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "boolean",
                    facet,
                }),
            "unsignedInt" => {
                self.unsigned_int(ctx, params)
                    .map_err(|facet| XsdDatatypeError::Facet {
                        type_name: "unsignedInt",
                        facet,
                    })
            }
            "ID" => self
                .id(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "ID",
                    facet,
                }),
            "IDREF" => self
                .idref(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "IDREF",
                    facet,
                }),
            "unsignedLong" => {
                self.unsigned_long(ctx, params)
                    .map_err(|facet| XsdDatatypeError::Facet {
                        type_name: "unsignedLong",
                        facet,
                    })
            }
            "float" => self
                .float(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "float",
                    facet,
                }),
            "nonNegativeInteger" => {
                self.non_negative_integer(ctx, params)
                    .map_err(|facet| XsdDatatypeError::Facet {
                        type_name: "nonNegativeInteger",
                        facet,
                    })
            }
            "negativeInteger" => {
                self.negative_integer(ctx, params)
                    .map_err(|facet| XsdDatatypeError::Facet {
                        type_name: "negativeInteger",
                        facet,
                    })
            }
            "nonPositiveInteger" => {
                self.non_positive_integer(ctx, params)
                    .map_err(|facet| XsdDatatypeError::Facet {
                        type_name: "nonPositiveInteger",
                        facet,
                    })
            }
            "byte" => self
                .byte(ctx, params)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "byte",
                    facet,
                }),
            "unsignedByte" => {
                self.unsigned_byte(ctx, params)
                    .map_err(|facet| XsdDatatypeError::Facet {
                        type_name: "unsignedByte",
                        facet,
                    })
            }
            "base64Binary" => {
                self.base64_binary(ctx, params)
                    .map_err(|facet| XsdDatatypeError::Facet {
                        type_name: "base64Binary",
                        facet,
                    })
            }
            "hexBinary" => {
                self.hex_binary(ctx, params)
                    .map_err(|facet| XsdDatatypeError::Facet {
                        type_name: "hexBinary",
                        facet,
                    })
            }
            "gYear" => self
                .pattern_only(ctx, params, XsdDatatypes::GYear)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "gYear",
                    facet,
                }),
            "gYearMonth" => self
                .pattern_only(ctx, params, XsdDatatypes::GYearMonth)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "gYearMonth",
                    facet,
                }),
            "gMonth" => self
                .pattern_only(ctx, params, XsdDatatypes::GMonth)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "gMonth",
                    facet,
                }),
            "gMonthDay" => self
                .pattern_only(ctx, params, XsdDatatypes::GMonthDay)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "gMonthDay",
                    facet,
                }),
            "gDay" => self
                .pattern_only(ctx, params, XsdDatatypes::GDay)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "gDay",
                    facet,
                }),
            "Name" => self
                .length_only(ctx, params, XsdDatatypes::Name)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "Name",
                    facet,
                }),
            "QName" => {
                // data type="QName" (without value): validate QName syntax only
                if !params.is_empty() {
                    return Err(XsdDatatypeError::Facet {
                        type_name: "QName",
                        facet: FacetError::InvalidFacet(
                            ctx.convert_span(span),
                            "QName data type does not support facets".to_string(),
                        ),
                    });
                }
                Ok(XsdDatatypes::QNameData)
            }
            "ENTITY" | "ENTITIES" => self
                .length_only(ctx, params, XsdDatatypes::Entity)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "ENTITY",
                    facet,
                }),
            "time" => self
                .pattern_only(ctx, params, XsdDatatypes::Time)
                .map_err(|facet| XsdDatatypeError::Facet {
                    type_name: "time",
                    facet,
                }),
            _ => Err(XsdDatatypeError::UnsupportedDatatype {
                span: ctx.convert_span(span),
                name: name.to_string(),
            }),
        }
    }

    fn compile_value(
        &self,
        ctx: &Context,
        span: &types::Span,
        name: &str,
        value: &str,
        ns: &[(String, String)],
    ) -> Result<XsdDatatypeValues, XsdDatatypeError> {
        match name {
            "string" => Ok(XsdDatatypeValues::String(value.to_string())),
            "token" => Ok(XsdDatatypeValues::Token(normalize_whitespace(value))),
            "QName" => Ok(XsdDatatypeValues::QName(
                QNameVal::from_val_with_ns_slice(value, ns).map_err(|_| {
                    XsdDatatypeError::InvalidValueOfType {
                        span: ctx.convert_span(span),
                        type_name: "QName",
                    }
                })?,
            )),
            _ => unimplemented!("{:?} not yet supported", name),
        }
    }

    fn normalized_string(
        &self,
        ctx: &Context,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, FacetError> {
        let mut len = LengthFacet::Unbounded;
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "length" => len.merge(LengthFacet::Length(Self::usize(ctx, param)?))?,
                "minLength" => len.merge(LengthFacet::MinLength(Self::usize(ctx, param)?))?,
                "maxLength" => len.merge(LengthFacet::MaxLength(Self::usize(ctx, param)?))?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::NormalizedString(StringFacets {
            len,
            pattern,
        }))
    }

    fn string(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut len = LengthFacet::Unbounded;
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "length" => len.merge(LengthFacet::Length(Self::usize(ctx, param)?))?,
                "minLength" => len.merge(LengthFacet::MinLength(Self::usize(ctx, param)?))?,
                "maxLength" => len.merge(LengthFacet::MaxLength(Self::usize(ctx, param)?))?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::String(StringFacets { len, pattern }))
    }

    fn short(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::i16(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::i16(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::i16(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::i16(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Short(min_max, pattern))
    }

    fn unsigned_short(
        &self,
        ctx: &Context,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::u16(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::u16(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::u16(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::u16(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::UnsignedShort(min_max, pattern))
    }

    fn long(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::i64(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::i64(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::i64(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::i64(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Long(min_max, pattern))
    }

    fn int(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::i32(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::i32(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::i32(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::i32(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Int(min_max, pattern))
    }
    fn integer(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::bigint(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::bigint(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::bigint(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::bigint(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Integer(min_max, pattern))
    }

    fn positive_integer(
        &self,
        ctx: &Context,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::biguint(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::biguint(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::biguint(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::biguint(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::PositiveInteger(min_max, pattern))
    }

    fn decimal(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;
        let mut fraction_digits = None;
        let mut total_digits = None;

        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::bigdecimal(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::bigdecimal(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::bigdecimal(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::bigdecimal(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                "fractionDigits" => fraction_digits = Some(Self::u16(ctx, param)?),
                "totalDigits" => total_digits = Some(Self::u16(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Decimal {
            min_max,
            pattern,
            fraction_digits,
            total_digits,
        })
    }
    fn double(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::f64(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::f64(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::f64(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::f64(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Double(pattern))
    }

    fn nmtokens(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut len = LengthFacet::Unbounded;

        for param in params {
            match &param.2.to_string()[..] {
                "length" => len.merge(LengthFacet::Length(Self::usize(ctx, param)?))?,
                "minLength" => len.merge(LengthFacet::MinLength(Self::usize(ctx, param)?))?,
                "maxLength" => len.merge(LengthFacet::MaxLength(Self::usize(ctx, param)?))?,
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::NmTokens(len))
    }

    fn nmtoken(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut len = LengthFacet::Unbounded;

        for param in params {
            match &param.2.to_string()[..] {
                "length" => len.merge(LengthFacet::Length(Self::usize(ctx, param)?))?,
                "minLength" => len.merge(LengthFacet::MinLength(Self::usize(ctx, param)?))?,
                "maxLength" => len.merge(LengthFacet::MaxLength(Self::usize(ctx, param)?))?,
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::NmToken(len))
    }

    fn ncname(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut len = LengthFacet::Unbounded;

        for param in params {
            match &param.2.to_string()[..] {
                "length" => len.merge(LengthFacet::Length(Self::usize(ctx, param)?))?,
                "minLength" => len.merge(LengthFacet::MinLength(Self::usize(ctx, param)?))?,
                "maxLength" => len.merge(LengthFacet::MaxLength(Self::usize(ctx, param)?))?,
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::NcName(len))
    }

    fn token(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut len = LengthFacet::Unbounded;

        for param in params {
            match &param.2.to_string()[..] {
                "length" => len.merge(LengthFacet::Length(Self::usize(ctx, param)?))?,
                "minLength" => len.merge(LengthFacet::MinLength(Self::usize(ctx, param)?))?,
                "maxLength" => len.merge(LengthFacet::MaxLength(Self::usize(ctx, param)?))?,
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Token(len))
    }

    fn duration(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Duration(pattern))
    }

    fn date(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Date(pattern))
    }

    fn datetime(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Datetime(pattern))
    }

    fn any_uri(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::AnyURI(pattern))
    }

    fn language(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Language(pattern))
    }

    fn boolean(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Boolean(pattern))
    }

    fn unsigned_int(
        &self,
        ctx: &Context,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::u32(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::u32(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::u32(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::u32(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::UnsignedInt(min_max, pattern))
    }

    fn unsigned_long(
        &self,
        ctx: &Context,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::u64(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::u64(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::u64(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::u64(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::UnsignedLong(min_max, pattern))
    }

    fn id(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::Id(pattern))
    }

    fn idref(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut pattern = None;

        for param in params {
            match &param.2.to_string()[..] {
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }

        Ok(XsdDatatypes::IdRef(pattern))
    }

    fn float(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut pattern = None;
        for param in params {
            match &param.2.to_string()[..] {
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }
        Ok(XsdDatatypes::Float(pattern))
    }

    fn non_negative_integer(
        &self,
        ctx: &Context,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;
        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::biguint(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::biguint(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::biguint(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::biguint(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }
        Ok(XsdDatatypes::NonNegativeInteger(min_max, pattern))
    }

    fn negative_integer(
        &self,
        ctx: &Context,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;
        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::bigint(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::bigint(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::bigint(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::bigint(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }
        Ok(XsdDatatypes::NegativeInteger(min_max, pattern))
    }

    fn non_positive_integer(
        &self,
        ctx: &Context,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;
        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::bigint(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::bigint(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::bigint(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::bigint(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }
        Ok(XsdDatatypes::NonPositiveInteger(min_max, pattern))
    }

    fn byte(&self, ctx: &Context, params: &[types::Param]) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;
        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::i8(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::i8(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::i8(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::i8(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }
        Ok(XsdDatatypes::Byte(min_max, pattern))
    }

    fn unsigned_byte(
        &self,
        ctx: &Context,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, FacetError> {
        let mut min_max = MinMaxFacet::default();
        let mut pattern = None;
        for param in params {
            match &param.2.to_string()[..] {
                "minInclusive" => min_max.min_inclusive(Self::u8(ctx, param)?)?,
                "minExclusive" => min_max.min_exclusive(Self::u8(ctx, param)?)?,
                "maxInclusive" => min_max.max_inclusive(Self::u8(ctx, param)?)?,
                "maxExclusive" => min_max.max_exclusive(Self::u8(ctx, param)?)?,
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }
        Ok(XsdDatatypes::UnsignedByte(min_max, pattern))
    }

    fn base64_binary(
        &self,
        ctx: &Context,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, FacetError> {
        let mut len = LengthFacet::Unbounded;
        for param in params {
            match &param.2.to_string()[..] {
                "length" => len.merge(LengthFacet::Length(Self::usize(ctx, param)?))?,
                "minLength" => len.merge(LengthFacet::MinLength(Self::usize(ctx, param)?))?,
                "maxLength" => len.merge(LengthFacet::MaxLength(Self::usize(ctx, param)?))?,
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }
        Ok(XsdDatatypes::Base64Binary(len))
    }

    fn hex_binary(
        &self,
        ctx: &Context,
        params: &[types::Param],
    ) -> Result<XsdDatatypes, FacetError> {
        let mut len = LengthFacet::Unbounded;
        for param in params {
            match &param.2.to_string()[..] {
                "length" => len.merge(LengthFacet::Length(Self::usize(ctx, param)?))?,
                "minLength" => len.merge(LengthFacet::MinLength(Self::usize(ctx, param)?))?,
                "maxLength" => len.merge(LengthFacet::MaxLength(Self::usize(ctx, param)?))?,
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }
        Ok(XsdDatatypes::HexBinary(len))
    }

    fn pattern_only(
        &self,
        ctx: &Context,
        params: &[types::Param],
        make: fn(Option<PatternFacet>) -> XsdDatatypes,
    ) -> Result<XsdDatatypes, FacetError> {
        let mut pattern = None;
        for param in params {
            match &param.2.to_string()[..] {
                "pattern" => pattern = Some(self.pattern(ctx, param)?),
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }
        Ok(make(pattern))
    }

    fn length_only(
        &self,
        ctx: &Context,
        params: &[types::Param],
        make: fn(LengthFacet) -> XsdDatatypes,
    ) -> Result<XsdDatatypes, FacetError> {
        let mut len = LengthFacet::Unbounded;
        for param in params {
            match &param.2.to_string()[..] {
                "length" => len.merge(LengthFacet::Length(Self::usize(ctx, param)?))?,
                "minLength" => len.merge(LengthFacet::MinLength(Self::usize(ctx, param)?))?,
                "maxLength" => len.merge(LengthFacet::MaxLength(Self::usize(ctx, param)?))?,
                _ => {
                    return Err(FacetError::InvalidFacet(
                        ctx.convert_span(&param.0),
                        param.2.to_string(),
                    ));
                }
            }
        }
        Ok(make(len))
    }

    fn i8(ctx: &Context, param: &types::Param) -> Result<i8, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn u8(ctx: &Context, param: &types::Param) -> Result<u8, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn i16(ctx: &Context, param: &types::Param) -> Result<i16, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn i32(ctx: &Context, param: &types::Param) -> Result<i32, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn i64(ctx: &Context, param: &types::Param) -> Result<i64, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn u32(ctx: &Context, param: &types::Param) -> Result<u32, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn u64(ctx: &Context, param: &types::Param) -> Result<u64, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn u16(ctx: &Context, param: &types::Param) -> Result<u16, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn f64(ctx: &Context, param: &types::Param) -> Result<f64, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: std::num::ParseFloatError| {
                FacetError::InvalidFloat(ctx.convert_span(&param.0), e.to_string())
            })
            .and_then(|v: f64| {
                if v.is_finite() {
                    Ok(v)
                } else {
                    Err(FacetError::InvalidFloat(
                        ctx.convert_span(&param.0),
                        "Only finite values allowed".to_string(),
                    ))
                }
            })
    }

    fn bigint(ctx: &Context, param: &types::Param) -> Result<num_bigint::BigInt, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: num_bigint::ParseBigIntError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn biguint(ctx: &Context, param: &types::Param) -> Result<num_bigint::BigUint, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: num_bigint::ParseBigIntError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn bigdecimal(
        ctx: &Context,
        param: &types::Param,
    ) -> Result<bigdecimal::BigDecimal, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: bigdecimal::ParseBigDecimalError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn usize(ctx: &Context, param: &types::Param) -> Result<usize, FacetError> {
        param
            .3
            .as_string_value()
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                FacetError::InvalidInt(ctx.convert_span(&param.0), e.to_string())
            })
    }

    fn pattern(&self, ctx: &Context, param: &types::Param) -> Result<PatternFacet, FacetError> {
        let raw = param.3.as_string_value();
        // XSD spec: pattern facet must match the entire lexical value (implicit ^ and $).
        let anchored = format!("^(?:{raw})$");
        regex::Regex::new(&anchored)
            .map(|re| PatternFacet(raw, re))
            .map_err(|e| FacetError::InvalidPattern(ctx.convert_span(&param.0), e))
    }
}

/// XSD QName value in the value-space: (namespace_uri, local_name).
/// The namespace_uri is "" for names in no namespace.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct QNameVal(String, String);

impl QNameVal {
    /// Resolve a QName string using a slice of (prefix, namespace_uri) bindings.
    pub(crate) fn from_val_with_ns_slice(val: &str, ns: &[(String, String)]) -> Result<Self, ()> {
        if let Some(pos) = val.find(':') {
            let prefix = &val[0..pos];
            let localname = &val[pos + 1..];
            if is_valid_ncname(prefix) && is_valid_ncname(localname) {
                let ns_uri = ns
                    .iter()
                    .find(|(p, _)| p == prefix)
                    .map(|(_, u)| u.as_str())
                    .ok_or(())?;
                Ok(QNameVal(ns_uri.to_string(), localname.to_string()))
            } else {
                Err(())
            }
        } else if is_valid_ncname(val) {
            let default_ns = ns
                .iter()
                .find(|(p, _)| p.is_empty())
                .map(|(_, u)| u.as_str())
                .unwrap_or("");
            Ok(QNameVal(default_ns.to_string(), val.to_string()))
        } else {
            Err(())
        }
    }

    /// Resolve a QName string using a dynamic namespace context (for instance validation).
    pub(crate) fn from_val_with_dyn_ns(
        val: &str,
        ns: &dyn super::Namespaces,
    ) -> Result<Self, ()> {
        if let Some(pos) = val.find(':') {
            let prefix = &val[0..pos];
            let localname = &val[pos + 1..];
            if is_valid_ncname(prefix) && is_valid_ncname(localname) {
                let ns_uri = ns.resolve(prefix).ok_or(())?;
                Ok(QNameVal(ns_uri.to_string(), localname.to_string()))
            } else {
                Err(())
            }
        } else if is_valid_ncname(val) {
            let default_ns = ns.resolve("").unwrap_or("");
            Ok(QNameVal(default_ns.to_string(), val.to_string()))
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_matches::assert_matches;
    use codemap::CodeMap;
    use relaxng_syntax::types;

    #[test]
    fn it_works() {
        let mut map = CodeMap::new();
        let file = map.add_file("main.rnc".to_string(), "just testing".to_string());
        let ctx = Context::new(file);
        let c = Compiler;
        let name =
            types::IdentifierOrKeyword::Identifier(types::Identifier(0..0, "length".to_string()));
        let value = types::Literal(
            0..0,
            vec![types::LiteralSegment {
                body: "1".to_string(),
            }],
        );
        let param = types::Param(0..0, None, name, value);
        let res = c.compile(&ctx, &(0..0), "normalizedString", &[param]);
        assert_matches!(
            res,
            Ok(XsdDatatypes::NormalizedString(StringFacets {
                len: LengthFacet::Length(1),
                pattern: None
            }))
        )
    }
}
