/// 通過語法與語意驗證的 main 主體。
pub(crate) struct Program {
    pub(crate) statements: Vec<Statement>,
}

/// 此版本支援的值型別；所有整數皆固定為 i32。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Type {
    I32,
    Bool,
}

/// 已解析 binding 與型別的敘述；id 在整個程式內唯一。
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
    Print {
        parts: Vec<PrintPart>,
        arguments: Vec<Expression>,
    },
    /// 帶分號且捨棄值的運算式，仍必須執行算術檢查。
    Evaluate(Expression),
}

/// println! 格式已解析為文字與參數索引；文字不含 NUL。
pub(crate) enum PrintPart {
    Text(String),
    Argument(usize),
}

/// 每個運算式皆帶有經語意分析確定的型別。
pub(crate) struct Expression {
    pub(crate) ty: Type,
    pub(crate) kind: ExpressionKind,
}

/// 純值運算式；不含賦值、函式呼叫或有副作用的區塊。
pub(crate) enum ExpressionKind {
    Integer(i32),
    Boolean(bool),
    Variable(usize),
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
