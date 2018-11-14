use std::fmt;

#[derive(Debug)]
pub enum ErrorKind {
    ExtensionNotSupported(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ErrorKind::ExtensionNotSupported(ref name) =>
                write!(f, "Extensions definined in SQL not supported (found {}). Please define extensions within the project file.", name),
        }
    }
}

#[derive(Debug)]
pub enum Statement {
    Error(ErrorKind),
    Function(FunctionDefinition),
    Index(IndexDefinition),
    Schema(SchemaDefinition),
    Table(TableDefinition),
    Type(TypeDefinition),
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum SqlType {
    Simple(SimpleSqlType),
    Array(SimpleSqlType, u32),
    Custom(ObjectName, Option<String>),
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
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
    Double,            // double precision
    Single,            // real
    Money,             // money

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
    Boolean(bool),
    Integer(i32),
    String(String),
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
    Base,
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
    pub name: String,
    pub sql_type: SqlType,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum FunctionReturnType {
    Table(Vec<ColumnDefinition>),
    SqlType(SqlType),
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum FunctionLanguage {
    C,
    Internal,
    PostgreSQL,
    SQL,
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
        match *self {
            AnyValue::Boolean(ref b) => write!(f, "{}", b),
            AnyValue::Integer(ref i) => write!(f, "{}", i),
            AnyValue::String(ref s) => write!(f, "'{}'", s),
        }
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
        match *self {
            SqlType::Simple(ref simple_type) => write!(f, "{}", simple_type),
            SqlType::Array(ref simple_type, dim) => write!(
                f,
                "{}{}",
                simple_type,
                (0..dim).map(|_| "[]").collect::<String>()
            ),
            SqlType::Custom(ref custom_type, ref options) => if let Some(ref opt) = *options {
                write!(f, "{}({})", custom_type, opt)
            } else {
                write!(f, "{}", custom_type)
            },
        }
    }
}

impl fmt::Display for TypeDefinitionKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TypeDefinitionKind::Enum(_) => write!(f, "Enum"),
            TypeDefinitionKind::Composite => write!(f, "Composite"),
            TypeDefinitionKind::Range => write!(f, "Range"),
            TypeDefinitionKind::Base => write!(f, "Base"),
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
