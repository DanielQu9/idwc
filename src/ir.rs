/// 通過語法與語意驗證的 main 主體及輔助函式。
pub(crate) struct Program {
    pub(crate) statements: Vec<Statement>,
    pub(crate) functions: Vec<Function>,
}

/// 已驗證的輔助函式；函式名稱以獨立 ID 表示。
pub(crate) struct Function {
    pub(crate) id: usize,
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) return_type: Type,
    pub(crate) statements: Vec<Statement>,
}

/// 函式參數的 binding 與值型別。
pub(crate) struct Parameter {
    pub(crate) id: usize,
    pub(crate) ty: Type,
    pub(crate) mutable: bool,
}

/// 此版本支援的值型別；所有整數皆固定為 i32。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Type {
    I32,
    Bool,
    Unit,
}

/// 已解析 binding 與型別的敘述；binding id 在所屬函式內唯一。
pub(crate) enum Statement {
    Let {
        id: usize,
        mutable: bool,
        value: Expression,
    },
    Assign {
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
    Argument(usize),
}

/// 每個運算式皆帶有經語意分析確定的型別。
pub(crate) struct Expression {
    pub(crate) ty: Type,
    pub(crate) kind: ExpressionKind,
}

/// 值運算式；呼叫與輸入可能有副作用，必須依序求值。
pub(crate) enum ExpressionKind {
    Integer(i32),
    Boolean(bool),
    Variable(usize),
    Unit,
    Call(usize, Vec<Expression>),
    ReadI32,
    FlushStdout,
    Unary(UnaryOp, Box<Expression>),
    Binary(BinaryOp, Box<Expression>, Box<Expression>),
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
