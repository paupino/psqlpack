use std::fmt::{self};

pub enum Statement {
    Extension(ExtensionDefinition),
    Function(FunctionDefinition),
    Schema(SchemaDefinition),
    Table(TableDefinition),
    Type(TypeDefinition),
}

#[derive(Serialize,Deserialize)]
pub enum SqlType {
    Simple(SimpleSqlType),
    Array(SimpleSqlType, u32),
    Custom(String, Option<String>),
}

#[derive(Serialize,Deserialize)]
pub enum SimpleSqlType {
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

#[derive(Serialize,Deserialize)]
pub enum ColumnConstraint {
    Default(AnyValue),
    NotNull,
    Null,
    Unique,
    PrimaryKey,
}

#[derive(Serialize,Deserialize)]
pub enum AnyValue {
    Boolean(bool),
    Integer(i32),
    String(String),
}

#[derive(Serialize,Deserialize)]
pub enum IndexParameter {
    FillFactor(u32),
}

#[derive(Serialize,Deserialize)]
pub enum TableConstraint {
    Primary { 
        name: String, 
        columns: Vec<String>, 
        parameters: Option<Vec<IndexParameter>> 
    },
    Foreign { 
        name: String, 
        columns: Vec<String>, 
        ref_table: ObjectName, 
        ref_columns: Vec<String>,
        match_type: Option<ForeignConstraintMatchType>,
        events: Option<Vec<ForeignConstraintEvent>>,
    },
}

#[derive(Serialize,Deserialize)]
pub enum ForeignConstraintMatchType {
    Simple,
    Partial,
    Full,
}

#[derive(Serialize,Deserialize)]
pub enum ForeignConstraintEvent {
    Delete(ForeignConstraintAction),
    Update(ForeignConstraintAction),
}

#[derive(Serialize,Deserialize)]
pub enum ForeignConstraintAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

#[derive(Serialize,Deserialize)]
pub struct TableDefinition {
    pub name: ObjectName, 
    pub columns: Vec<ColumnDefinition>, 
    pub constraints: Option<Vec<TableConstraint>>,
}

#[derive(Serialize,Deserialize)]
pub struct ObjectName {
    pub schema: Option<String>,
    pub name: String,
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

#[derive(Serialize,Deserialize)]
pub struct ExtensionDefinition {
    pub name: String,
}

#[derive(Serialize,Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub kind: TypeDefinitionKind,
}

#[derive(Serialize,Deserialize)]
pub enum TypeDefinitionKind {
    Alias(SqlType),
    Enum(Vec<String>),
}

#[derive(Serialize,Deserialize)]
pub struct ScriptDefinition {
    pub name: String,
    pub kind: ScriptKind,
    pub order: usize,
    pub contents: String,
}

#[derive(Serialize,Deserialize)]
pub enum ScriptKind {
    PreDeployment,
    PostDeployment
}

#[derive(Serialize,Deserialize)]
pub struct FunctionDefinition {
    pub name: ObjectName,
    pub arguments: Vec<FunctionArgument>,
    pub return_type: FunctionReturnType,
    pub body: String,
    pub language: FunctionLanguage,
}

#[derive(Serialize,Deserialize)]
pub struct FunctionArgument {
    pub name: String,
    pub sql_type: SqlType,
}

#[derive(Serialize,Deserialize)]
pub enum FunctionReturnType {
    Table(Vec<ColumnDefinition>),
    SqlType(SqlType),
}

#[derive(Serialize,Deserialize)]
pub enum FunctionLanguage {
    C,
    Internal,
    PostgreSQL,
    SQL,
}