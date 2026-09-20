/// 通過語法與語意驗證的 main 主體及輔助函式。
pub(crate) struct Program {
    pub(crate) statements: Vec<Statement>,
    pub(crate) functions: Vec<Function>,
}

/// 已驗證的輔助函式；函式名稱以獨立 ID 表示。
pub(crate) struct Function {
    pub(crate) id: usize,
    pub(crate) name: String,
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) return_type: Type,
    pub(crate) statements: Vec<Statement>,
}

/// 函式參數的 binding 與值型別。
pub(crate) struct Parameter {
    pub(crate) id: usize,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) mutable: bool,
}

/// 此版本支援的 scalar、一維固定陣列，以及限定的行輸入中介型別。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Type {
    I32,
    Usize,
    F64,
    Bool,
    Unit,
    Array(ArrayElement, usize),
    Vec(ArrayElement),
    Str,
    String,
    Tokens,
}

/// 固定陣列允許的元素型別；不包含巢狀陣列或 unit。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArrayElement {
    I32,
    Usize,
    F64,
    Bool,
}

impl ArrayElement {
    pub(crate) fn ty(self) -> Type {
        match self {
            Self::I32 => Type::I32,
            Self::Usize => Type::Usize,
            Self::F64 => Type::F64,
            Self::Bool => Type::Bool,
        }
    }
}

/// 已解析 binding 與型別的敘述；binding id 在所屬函式內唯一。
pub(crate) enum Statement {
    Let {
        id: usize,
        name: String,
        mutable: bool,
        value: Expression,
    },
    Assign {
        id: usize,
        value: Expression,
    },
    AssignIndex {
        id: usize,
        length: usize,
        index: Expression,
        value: Expression,
        op: Option<BinaryOp>,
    },
    VecAssignIndex {
        id: usize,
        index: Expression,
        value: Expression,
    },
    VecPush {
        id: usize,
        value: Expression,
    },
    Block(Vec<Statement>),
    /// 所有分支均為 unit 敘述；else if 表示為 else 分支中的 If。
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    /// condition 必須在每次迭代（包含 continue 後）重新求值。
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    /// 整數範圍上下限只求值一次；每次迭代建立新的 loop binding。
    ForRange {
        id: usize,
        name: String,
        ty: Type,
        mutable: bool,
        start: Expression,
        end: Expression,
        inclusive: bool,
        body: Vec<Statement>,
    },
    Loop(Vec<Statement>),
    /// 無標籤跳躍；只作用於最內層迴圈。
    Break,
    Continue,
    Print {
        parts: Vec<PrintPart>,
        arguments: Vec<Expression>,
        newline: bool,
    },
    Return(Option<Expression>),
    /// 帶分號且捨棄值的運算式，仍必須執行算術檢查。
    Evaluate(Expression),
}

/// print!／println! 格式已解析為文字與參數索引；文字不含 NUL。
pub(crate) enum PrintPart {
    Text(String),
    Argument { index: usize, format: PrintFormat },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrintFormat {
    Display,
    Precision(u8),
    Debug,
}

/// 每個運算式皆帶有經語意分析確定的型別。
pub(crate) struct Expression {
    pub(crate) ty: Type,
    pub(crate) kind: ExpressionKind,
}

/// 值運算式；呼叫與輸入可能有副作用，必須依序求值。
pub(crate) enum ExpressionKind {
    Integer(i32),
    Usize(usize),
    Float(f64),
    Boolean(bool),
    Variable(usize),
    Array(Vec<Expression>),
    ArrayRepeat(Box<Expression>, usize),
    Index {
        id: usize,
        length: usize,
        index: Box<Expression>,
    },
    ArrayLength(usize),
    Vec(Vec<Expression>),
    VecRepeat(Box<Expression>, usize),
    VecNew,
    VecIndex {
        id: usize,
        index: Box<Expression>,
    },
    VecLength(usize),
    StringLiteral(String),
    StringNew,
    StringFrom(Box<Expression>),
    ReadString,
    ReadLine(usize),
    SplitWhitespace(usize),
    TokensLength(usize),
    Parse {
        source: ParseSource,
    },
    Powi2(Box<Expression>),
    Unit,
    Call(usize, Vec<Expression>),
    ReadI32,
    ReadF64,
    FlushStdout,
    Unary(UnaryOp, Box<Expression>),
    Binary(BinaryOp, Box<Expression>, Box<Expression>),
}

/// v0.7.0 限定解析來源；不向 IR 暴露一般 `&str` 或 iterator。
pub(crate) enum ParseSource {
    TrimmedString(usize),
    Token { id: usize, index: Box<Expression> },
}

/// 一元算術負號與布林反轉。
#[derive(Clone, Copy)]
pub(crate) enum UnaryOp {
    Negate,
    Not,
}

/// 經型別檢查的二元運算；And 與 Or 必須保留短路求值。
#[derive(Clone, Copy)]
pub(crate) enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}
