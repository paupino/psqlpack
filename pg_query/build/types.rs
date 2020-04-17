use serde::de::{Deserialize, Deserializer, MapAccess, Visitor, Error};

use std::fmt;
use serde::export::Formatter;

pub struct Struct {
    pub fields: Vec<Field>,
    pub comment: Option<String>,
}

impl<'de> Deserialize<'de> for Struct {
    fn deserialize<D>(d: D) -> Result<Struct, D::Error>
        where D: Deserializer<'de>
    {
        enum StructField {
            Fields,
            Comment,
        }

        impl<'de> Deserialize<'de> for StructField {
            fn deserialize<D>(d: D) -> Result<StructField, D::Error>
                where D: Deserializer<'de>
            {
                struct StructFieldVisitor;

                impl<'de> Visitor<'de> for StructFieldVisitor {
                    type Value = StructField;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        write!(formatter, "a string that represents an StructField")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<StructField, E>
                        where E: Error
                    {
                        match value {
                            "fields" => Ok(StructField::Fields),
                            "comment" => Ok(StructField::Comment),
                            f => Err(E::unknown_field(f, &["fields", "comment"])),
                        }
                    }
                }

                d.deserialize_str(StructFieldVisitor)
            }
        }

        struct StructVisitor;

        impl<'de> Visitor<'de> for StructVisitor {
            type Value = Struct;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a map that represents a Struct")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Struct, A::Error>
                where A: MapAccess<'de>
            {
                let mut fields = None;
                let mut comment = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        StructField::Fields => {
                            let v: Vec<Field> = map.next_value()?;
                            fields = Some(v)
                        },
                        StructField::Comment => {
                            let v: Option<String> = map.next_value()?;
                            comment = Some(v);
                        }
                    }
                }

                let fields = match fields {
                    Some(fields) => fields.into_iter()
                        .filter(|f| f.name.is_some())
                        .collect(),
                    None => return Err(A::Error::missing_field("fields")),
                };
                let comment = match comment {
                    Some(comment) => comment,
                    None => return Err(A::Error::missing_field("comment")),
                };

                Ok(Struct {
                    fields,
                    comment,
                })
            }
        }

        d.deserialize_map(StructVisitor)
    }
}

pub struct Field {
    pub name: Option<String>,
    pub c_type: Option<String>,
    pub comment: Option<String>,
}

impl<'de> Deserialize<'de> for Field {
    fn deserialize<D>(d: D) -> Result<Field, D::Error>
        where D: Deserializer<'de>
    {
        enum FieldField {
            Name,
            CType,
            Comment,
        }

        impl<'de> Deserialize<'de> for FieldField {
            fn deserialize<D>(d: D) -> Result<FieldField, D::Error>
                where D: Deserializer<'de>
            {
                struct FieldFieldVisitor;

                impl<'de> Visitor<'de> for FieldFieldVisitor {
                    type Value = FieldField;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        write!(formatter, "a string that represents a FieldField")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<FieldField, E>
                        where E: Error
                    {
                        match value {
                            "name" => Ok(FieldField::Name),
                            "c_type" => Ok(FieldField::CType),
                            "comment" => Ok(FieldField::Comment),
                            f => Err(E::unknown_field(f, &["name", "c_type", "comment"])),
                        }
                    }
                }

                d.deserialize_str(FieldFieldVisitor)
            }
        }

        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a map that represents a Field")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Field, A::Error>
                where A: MapAccess<'de>
            {
                let mut name = None;
                let mut c_type = None;
                let mut comment = None;

                while let Some((key, value)) = map.next_entry()? {
                    match key {
                        FieldField::Name => name = Some(value),
                        FieldField::CType => c_type = Some(value),
                        FieldField::Comment => comment = Some(value),
                    }
                }

                let name = match name {
                    Some(name) => name,
                    None => None,
                };
                let c_type = match c_type {
                    Some(c_type) => c_type,
                    None => None,
                };
                let comment = match comment {
                    Some(comment) => comment,
                    None => None,
                };

                Ok(Field {
                    name,
                    c_type,
                    comment,
                })
            }
        }

        d.deserialize_map(FieldVisitor)
    }
}

pub struct Enum {
    pub values: Vec<EnumValue>,
    pub comment: Option<String>,
}

impl<'de> Deserialize<'de> for Enum {
    fn deserialize<D>(d: D) -> Result<Enum, D::Error>
        where D: Deserializer<'de>
    {
        enum EnumField {
            Values,
            Comment,
        }

        impl<'de> Deserialize<'de> for EnumField {
            fn deserialize<D>(d: D) -> Result<EnumField, D::Error>
                where D: Deserializer<'de>
            {
                struct EnumFieldVisitor;

                impl<'de> Visitor<'de> for EnumFieldVisitor {
                    type Value = EnumField;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        write!(formatter, "a string that represents an EnumField")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<EnumField, E>
                        where E: Error
                    {
                        match value {
                            "values" => Ok(EnumField::Values),
                            "comment" => Ok(EnumField::Comment),
                            f => Err(E::unknown_field(f, &["values", "comment"])),
                        }
                    }
                }

                d.deserialize_str(EnumFieldVisitor)
            }
        }

        struct EnumVisitor;

        impl<'de> Visitor<'de> for EnumVisitor {
            type Value = Enum;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a map that represents an Enum")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Enum, A::Error>
                where A: MapAccess<'de>
            {
                let mut values = None;
                let mut comment = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        EnumField::Values => {
                            let v: Vec<EnumValue> = map.next_value()?;
                            values = Some(v);
                        },
                        EnumField::Comment => {
                            let v: Option<String> = map.next_value()?;
                            comment = Some(v);
                        }
                    }
                }

                let values = match values {
                    Some(values) => values.into_iter()
                        .filter(|v| v.name.is_some())
                        .collect(),
                    None => return Err(A::Error::missing_field("values")),
                };
                let comment = match comment {
                    Some(comment) => comment,
                    None => return Err(A::Error::missing_field("comment")),
                };

                Ok(Enum {
                    values,
                    comment,
                })
            }
        }

        d.deserialize_map(EnumVisitor)
    }
}

pub struct EnumValue {
    pub name: Option<String>,
    pub comment: Option<String>,
}

impl<'de> Deserialize<'de> for EnumValue {
    fn deserialize<D>(d: D) -> Result<EnumValue, D::Error>
        where D: Deserializer<'de>
    {
        enum EnumValueField {
            Name,
            Comment,
        }

        impl<'de> Deserialize<'de> for EnumValueField {
            fn deserialize<D>(d: D) -> Result<EnumValueField, D::Error>
                where D: Deserializer<'de>
            {
                struct EnumValueFieldVisitor;

                impl<'de> Visitor<'de> for EnumValueFieldVisitor {
                    type Value = EnumValueField;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        write!(formatter, "a string that represents an EnumValueField")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<EnumValueField, E>
                        where E: Error
                    {
                        match value {
                            "name" => Ok(EnumValueField::Name),
                            "comment" => Ok(EnumValueField::Comment),
                            f => Err(E::unknown_field(f, &["name", "comment"])),
                        }
                    }
                }

                d.deserialize_str(EnumValueFieldVisitor)
            }
        }

        struct EnumValueVisitor;

        impl<'de> Visitor<'de> for EnumValueVisitor {
            type Value = EnumValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a map that represents an EnumValue")
            }

            fn visit_map<A>(self, mut map: A) -> Result<EnumValue, A::Error>
                where A: MapAccess<'de>
            {
                let mut name = None;
                let mut comment = None;

                while let Some((key, value)) = map.next_entry()? {
                    match key {
                        EnumValueField::Name => {
                            name = Some(value);
                        },
                        EnumValueField::Comment => {
                            comment = Some(value);
                        },
                    }
                }

                let name = match name {
                    Some(name) => name,
                    None => None,
                };
                let comment = match comment {
                    Some(comment) => comment,
                    None => None,
                };

                Ok(EnumValue {
                    name,
                    comment,
                })
            }
        }

        d.deserialize_map(EnumValueVisitor)
    }
}