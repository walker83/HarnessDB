use types::DataType;

/// Reference to a column within a block, identified by index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColumnRef {
    pub index: usize,
    pub name: String,
}

impl ColumnRef {
    pub fn new(index: usize, name: impl Into<String>) -> Self {
        Self {
            index,
            name: name.into(),
        }
    }
}

/// Binary operator for comparing or combining two expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Logical
    And,
    Or,
    // String
    Like,
    NotLike,
    // Bitwise
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
}

/// Unary operator applied to a single expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    Not,
    Neg,
    BitwiseNot,
}

/// A fully qualified function call with a name and argument list.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    pub name: String,
    pub args: Vec<Expr>,
    pub distinct: bool,
}

impl FunctionCall {
    pub fn new(name: impl Into<String>, args: Vec<Expr>) -> Self {
        Self {
            name: name.into(),
            args,
            distinct: false,
        }
    }

    pub fn distinct(name: impl Into<String>, args: Vec<Expr>) -> Self {
        Self {
            name: name.into(),
            args,
            distinct: true,
        }
    }
}

/// A single WHEN/THEN branch inside a CASE expression.
#[derive(Debug, Clone, PartialEq)]
pub struct WhenThen {
    pub when: Expr,
    pub then: Expr,
}

/// The top-level expression tree node.
///
/// Every expression that the query engine can evaluate is represented by one
/// of these variants. The evaluator walks this tree against a `Block` of data
/// to produce a result `Vector`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Reference to an input column by index.
    ColumnRef(ColumnRef),

    /// A single constant value repeated for every row.
    Literal(types::ScalarValue),

    /// `left OP right`
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// `OP expr`
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expr>,
    },

    /// Named function call, e.g. `abs(x)`, `concat(a, b)`.
    FunctionCall(FunctionCall),

    /// Cast expression: `CAST(expr AS target_type)`.
    Cast {
        expr: Box<Expr>,
        target_type: DataType,
    },

    /// `expr IS NULL` / `expr IS NOT NULL`.
    IsNull {
        expr: Box<Expr>,
        negated: bool,
    },

    /// CASE WHEN ... THEN ... [ELSE ...] END.
    CaseWhen {
        cases: Vec<WhenThen>,
        else_expr: Option<Box<Expr>>,
    },

    /// `expr IN (val1, val2, ...)` or `expr NOT IN (...)`.
    InList {
        expr: Box<Expr>,
        list: Vec<Expr>,
        negated: bool,
    },

    /// `expr BETWEEN low AND high` or `expr NOT BETWEEN low AND high`.
    Between {
        expr: Box<Expr>,
        low: Box<Expr>,
        high: Box<Expr>,
        negated: bool,
    },

    /// `expr LIKE pattern` or `expr NOT LIKE pattern`.
    Like {
        expr: Box<Expr>,
        pattern: Box<Expr>,
        negated: bool,
    },

    /// EXISTS(subquery) -- placeholder for subquery support.
    Exists {
        negated: bool,
    },
}

impl Expr {
    // -- Convenience constructors -------------------------------------------

    pub fn column(index: usize, name: impl Into<String>) -> Self {
        Self::ColumnRef(ColumnRef::new(index, name))
    }

    pub fn literal(val: types::ScalarValue) -> Self {
        Self::Literal(val)
    }

    pub fn binary(op: BinaryOperator, left: Expr, right: Expr) -> Self {
        Self::BinaryOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    pub fn unary(op: UnaryOperator, expr: Expr) -> Self {
        Self::UnaryOp {
            op,
            expr: Box::new(expr),
        }
    }

    pub fn call(name: impl Into<String>, args: Vec<Expr>) -> Self {
        Self::FunctionCall(FunctionCall::new(name, args))
    }

    pub fn cast(expr: Expr, target_type: DataType) -> Self {
        Self::Cast {
            expr: Box::new(expr),
            target_type,
        }
    }

    pub fn is_null(expr: Expr) -> Self {
        Self::IsNull {
            expr: Box::new(expr),
            negated: false,
        }
    }

    pub fn is_not_null(expr: Expr) -> Self {
        Self::IsNull {
            expr: Box::new(expr),
            negated: true,
        }
    }

    pub fn in_list(expr: Expr, list: Vec<Expr>, negated: bool) -> Self {
        Self::InList {
            expr: Box::new(expr),
            list,
            negated,
        }
    }

    pub fn between(expr: Expr, low: Expr, high: Expr, negated: bool) -> Self {
        Self::Between {
            expr: Box::new(expr),
            low: Box::new(low),
            high: Box::new(high),
            negated,
        }
    }

    pub fn like(expr: Expr, pattern: Expr, negated: bool) -> Self {
        Self::Like {
            expr: Box::new(expr),
            pattern: Box::new(pattern),
            negated,
        }
    }
}
