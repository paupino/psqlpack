use rust_decimal::Decimal;

use std::fmt;

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    ExtensionNotSupported(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ErrorKind::ExtensionNotSupported(ref name) => write!(
                f,
                "Extensions defined in SQL not supported (found {}). Please define extensions within the project file.",
                name
            ),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Statement {
    Error(ErrorKind),
    Function(FunctionDefinition),
    Index(IndexDefinition),
    Schema(SchemaDefinition),
    Table(TableDefinition),
    Type(TypeDefinition),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum SqlType {
    Simple(SimpleSqlType, Option<u32>),              // type, dim
    Custom(ObjectName, Vec<TypeModifier>, Option<u32>), // type, options, dim
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum TypeModifier {
    Ident(String),
    Integer(i32),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum SimpleSqlType {
    FixedLengthString(u32),    // char(size)
    VariableLengthString(u32), // varchar(size)
    UnsizedVariableLengthString,
    Text, // text

    FixedLengthBitString(u32),    // bit(size)
    VariableLengthBitString(u32), // varbit(size)

    SmallInteger, // smallint
    Integer,      // int
    BigInteger,   // bigint

    SmallSerial, // smallserial
    Serial,      // serial
    BigSerial,   // bigserial

    Numeric(Option<(u32, u32)>), // numeric(m,d)
    Double,                      // double precision
    Single,                      // real
    Money,                       // money

    Boolean, // bool

    Date,                 // date
    DateTime,             // timestamp without time zone
    DateTimeWithTimeZone, // timestamp with time zone
    Time,                 // time
    TimeWithTimeZone,     // time with time zone

    Uuid, // uuid
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ColumnConstraint {
    Default(AnyValue),
    NotNull,
    Null,
    Unique,
    PrimaryKey,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum AnyValue {
    // Optional cast on each of these
    Array(Vec<AnyValue>, Option<SqlType>),
    Boolean(bool, Option<SqlType>),
    Decimal(Decimal, Option<SqlType>),
    Integer(i32, Option<SqlType>),
    String(String, Option<SqlType>),
    Null(Option<SqlType>),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum IndexParameter {
    FillFactor(u32),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum TableConstraint {
    Primary {
        name: String,
        columns: Vec<String>,
        parameters: Option<Vec<IndexParameter>>,
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

impl TableConstraint {
    pub fn name(&self) -> &str {
        match *self {
            TableConstraint::Primary { ref name, .. } | TableConstraint::Foreign { ref name, .. } => name,
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ForeignConstraintMatchType {
    Simple,
    Partial,
    Full,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ForeignConstraintEvent {
    Delete(ForeignConstraintAction),
    Update(ForeignConstraintAction),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ForeignConstraintAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone, Serialize, Deserialize)]
pub struct ObjectName {
    pub schema: Option<String>,
    pub name: String,
}

impl ObjectName {
    pub fn schema(&self) -> &str {
        if let Some(ref schema) = self.schema {
            schema
        } else {
            ""
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct TableDefinition {
    pub name: ObjectName,
    pub columns: Vec<ColumnDefinition>,
    pub constraints: Vec<TableConstraint>,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub sql_type: SqlType,
    pub constraints: Vec<ColumnConstraint>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SchemaDefinition {
    pub name: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub name: ObjectName,
    pub kind: TypeDefinitionKind,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum TypeDefinitionKind {
    Composite,
    Enum(Vec<String>),
    Range,
    UserDefined,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ScriptDefinition {
    pub name: String,
    pub kind: ScriptKind,
    pub order: usize,
    pub contents: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ScriptKind {
    PreDeployment,
    PostDeployment,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: ObjectName,
    pub arguments: Vec<FunctionArgument>,
    pub return_type: FunctionReturnType,
    pub body: String,
    pub language: FunctionLanguage,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct FunctionArgument {
    pub mode: Option<FunctionArgumentMode>,
    pub name: Option<String>,
    pub sql_type: SqlType,
    pub default: Option<AnyValue>,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum FunctionArgumentMode {
    In,
    InOut,
    Out,
    Variadic,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum FunctionReturnType {
    Table(Vec<ColumnDefinition>),
    SetOf(SqlType),
    SqlType(SqlType),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum FunctionLanguage {
    C,
    Internal,
    PostgreSQL,
    SQL,
    Custom(String),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct IndexDefinition {
    pub name: String,
    pub table: ObjectName,
    pub columns: Vec<IndexColumn>,

    pub unique: bool,
    pub index_type: Option<IndexType>,
    pub storage_parameters: Option<Vec<IndexParameter>>,
}

impl IndexDefinition {
    pub fn fully_qualified_name(&self) -> String {
        format!("{}.{}", self.schema(), self.name)
    }

    pub fn is_same_index(&self, other: &IndexDefinition) -> bool {
        self.name.eq(&other.name) && self.schema().eq(other.schema())
    }

    pub fn schema(&self) -> &str {
        self.table.schema()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    Gist,
    Gin,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct IndexColumn {
    pub name: String,
    pub order: Option<IndexOrder>,
    pub null_position: Option<IndexPosition>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum IndexOrder {
    Ascending,
    Descending,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum IndexPosition {
    First,
    Last,
}

impl fmt::Display for AnyValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sql_type = match *self {
            AnyValue::Array(ref items, ref sql_type) => {
                write!(f, "ARRAY [")?;
                let mut comma = false;
                for item in items {
                    if comma {
                        write!(f, ", ")?;
                    } else {
                        comma = true;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")?;
                sql_type
            }
            AnyValue::Boolean(ref b, ref sql_type) => {
                write!(f, "{}", b)?;
                sql_type
            }
            AnyValue::Decimal(ref d, ref sql_type) => {
                write!(f, "{}", d)?;
                sql_type
            }
            AnyValue::Integer(ref i, ref sql_type) => {
                write!(f, "{}", i)?;
                sql_type
            }
            AnyValue::String(ref s, ref sql_type) => {
                write!(f, "'{}'", s)?;
                sql_type
            }
            AnyValue::Null(ref sql_type) => {
                write!(f, "NULL")?;
                sql_type
            }
        };
        if let Some(sql_type) = sql_type {
            write!(f, "::{}", sql_type)?;
        }
        Ok(())
    }
}

impl fmt::Display for ForeignConstraintMatchType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ForeignConstraintMatchType::Simple => write!(f, "MATCH SIMPLE"),
            ForeignConstraintMatchType::Partial => write!(f, "MATCH PARTIAL"),
            ForeignConstraintMatchType::Full => write!(f, "MATCH FULL"),
        }
    }
}

impl fmt::Display for ForeignConstraintAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ForeignConstraintAction::NoAction => write!(f, "NO ACTION"),
            ForeignConstraintAction::Restrict => write!(f, "RESTRICT"),
            ForeignConstraintAction::Cascade => write!(f, "CASCADE"),
            ForeignConstraintAction::SetNull => write!(f, "SET NULL"),
            ForeignConstraintAction::SetDefault => write!(f, "SET DEFAULT"),
        }
    }
}

impl fmt::Display for FunctionArgument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref mode) = self.mode {
            write!(f, "{}", mode)?;
        }
        if let Some(ref name) = self.name {
            write!(f, "{} {}", name, self.sql_type)?;
        } else {
            write!(f, "{}", self.sql_type)?;
        }
        if let Some(ref default) = self.default {
            write!(f, "{}", default)?;
        }
        Ok(())
    }
}

impl fmt::Display for FunctionArgumentMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FunctionArgumentMode::In => write!(f, "IN"),
            FunctionArgumentMode::InOut => write!(f, "INOUT"),
            FunctionArgumentMode::Out => write!(f, "OUT"),
            FunctionArgumentMode::Variadic => write!(f, "VARIADIC"),
        }
    }
}

impl fmt::Display for FunctionLanguage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FunctionLanguage::C => write!(f, "C"),
            FunctionLanguage::Internal => write!(f, "INTERNAL"),
            FunctionLanguage::PostgreSQL => write!(f, "PLPGSQL"),
            FunctionLanguage::SQL => write!(f, "SQL"),
            FunctionLanguage::Custom(ref name) => write!(f, "{}", name),
        }
    }
}

impl fmt::Display for ObjectName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.schema {
            Some(ref s) => write!(f, "{}.{}", s, self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

impl fmt::Display for SqlType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn dimensions(dim: Option<u32>) -> String {
            if let Some(dim) = dim {
                (0..dim).map(|_| "[]").collect::<String>()
            } else {
                String::new()
            }
        }
        match *self {
            SqlType::Simple(ref simple_type, dim) => write!(f, "{}{}", simple_type, dimensions(dim)),
            SqlType::Custom(ref custom_type, ref modifiers, dim) => {
                if modifiers.is_empty() {
                    write!(f, "{}{}", custom_type, dimensions(dim))
                } else {
                    write!(f, "{}(", custom_type)?;
                    let mut first = true;
                    for modifier in modifiers {
                        if first {
                            first = false;
                        } else {
                            write!(f, ",")?;
                        }
                        match modifier {
                            TypeModifier::Ident(ident) => write!(f, "{}", ident)?,
                            TypeModifier::Integer(integer) => write!(f, "{}", integer)?,
                        }
                    }
                    write!(f, "){}", dimensions(dim))
                }
            }
        }
    }
}

impl fmt::Display for TypeDefinitionKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TypeDefinitionKind::Enum(_) => write!(f, "Enum"),
            TypeDefinitionKind::Composite => write!(f, "Composite"),
            TypeDefinitionKind::Range => write!(f, "Range"),
            TypeDefinitionKind::UserDefined => write!(f, "User Defined"),
        }
    }
}

impl fmt::Display for SimpleSqlType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SimpleSqlType::FixedLengthString(size) => write!(f, "char({})", size),
            SimpleSqlType::VariableLengthString(size) => write!(f, "varchar({})", size),
            SimpleSqlType::UnsizedVariableLengthString => write!(f, "varchar"),
            SimpleSqlType::Text => write!(f, "text"),

            SimpleSqlType::FixedLengthBitString(size) => write!(f, "bit({})", size),
            SimpleSqlType::VariableLengthBitString(size) => write!(f, "varbit({})", size),

            SimpleSqlType::SmallInteger => write!(f, "smallint"),
            SimpleSqlType::Integer => write!(f, "int"),
            SimpleSqlType::BigInteger => write!(f, "bigint"),

            SimpleSqlType::SmallSerial => write!(f, "smallserial"),
            SimpleSqlType::Serial => write!(f, "serial"),
            SimpleSqlType::BigSerial => write!(f, "bigserial"),

            SimpleSqlType::Numeric(Some((m, d))) => write!(f, "numeric({},{})", m, d),
            SimpleSqlType::Numeric(None) => write!(f, "numeric"),
            SimpleSqlType::Double => write!(f, "double precision"),
            SimpleSqlType::Single => write!(f, "real"),
            SimpleSqlType::Money => write!(f, "money"),

            SimpleSqlType::Boolean => write!(f, "bool"),

            SimpleSqlType::Date => write!(f, "date"),
            SimpleSqlType::DateTime => write!(f, "timestamp without time zone"),
            SimpleSqlType::DateTimeWithTimeZone => write!(f, "timestamp with time zone"),
            SimpleSqlType::Time => write!(f, "time"),
            SimpleSqlType::TimeWithTimeZone => write!(f, "time with time zone"),

            SimpleSqlType::Uuid => write!(f, "uuid"),
        }
    }
}
