use std::fmt::{self};

pub enum Statement {
    Table(TableDefinition),
    Schema(SchemaDefinition),
}

#[derive(Serialize,Deserialize)]
pub enum SqlType {
    FixedLengthString(u32), // char(size)
    VariableLengthString(u32), // varchar(size)
    Text, // text
    
    FixedLengthBitString(u32), // bit(size)
    VariableLengthBitString(u32), // varbit(size)

    SmallInteger, // smallint
    Integer, // int
    BigInteger, // bigint

    SmallSerial, // smallserial
    Serial, // serial
    BigSerial, // bigserial

    Numeric(u32, u32), // numeric(m,d)
    Double, // double precision
    Single, // real
    Money, // money

    Boolean, // bool

    Date, // date
    DateTime, // timestamp without time zone
    DateTimeWithTimeZone, // timestamp with time zone
    Time, // time
    TimeWithTimeZone, // time with time zone

    Uuid, // uuid

    Custom(String),
}

#[derive(Serialize,Deserialize)]
pub enum ColumnConstraint {
    NotNull,
    Null,
    Unique,
    PrimaryKey,
}

#[derive(Serialize,Deserialize)]
pub enum IndexParameter {
    FillFactor(u32),
}

#[derive(Serialize,Deserialize)]
pub enum TableConstraint {
    Primary { name: String, columns: Vec<String>, parameters: Option<Vec<IndexParameter>> },
    Foreign { name: String, columns: Vec<String>, ref_table: TableName, ref_columns: Vec<String> },
}

#[derive(Serialize,Deserialize)]
pub struct TableDefinition {
    pub name: TableName, 
    pub columns: Vec<ColumnDefinition>, 
    pub constraints: Option<Vec<TableConstraint>>,
}

#[derive(Serialize,Deserialize)]
pub struct TableName {
    pub schema: Option<String>,
    pub name: String,
}

impl fmt::Display for TableName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.schema {
            Some(ref s) => write!(f, "{}.{}", s, self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

#[derive(Serialize,Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub sql_type: SqlType,
    pub constraints: Option<Vec<ColumnConstraint>>,
}

#[derive(Serialize,Deserialize)]
pub struct SchemaDefinition {
    pub name: String,
}