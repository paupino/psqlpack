pub enum Statement {
    TableDefinition { name: TableName, columns: Vec<ColumnDefinition>, constraints: Option<Vec<Constraint>> }
}

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
}

pub enum Qualifier {
    NotNull,
    Null,
    Unique,
    PrimaryKey,
}

pub enum IndexOption {
    FillFactor(u32),
}

pub enum Constraint {
    Primary { name: String, columns: Vec<String>, options: Option<Vec<IndexOption>> },
    Foreign { name: String, columns: Vec<String>, ref_table: TableName, ref_columns: Vec<String> },
}

pub struct TableName {
    pub schema: Option<String>,
    pub name: String,
}

pub struct ColumnDefinition {
    pub name: String,
    pub sql_type: SqlType,
    pub qualifiers: Option<Vec<Qualifier>>,
}