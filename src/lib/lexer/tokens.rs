pub enum Token {
    EOF = -1,

    SELECT,
    FROM,
    WHERE,

    ORDER_BY,
    ASC,
    DESC,

    GROUP_BY,
    HAVING,

    LIMIT,
    OFFSET,

    INSERT,
    INTO,

    UPDATE,
    SET,

    DELETE,

    JOIN,
    INNER,
    LEFT,
    RIGHT,
    FULL,
    OUTER,

    CREATE,
    ALTER,
    DROP,

    TABLE,
    COLUMN,

    // primary
    IDENTIFIER(String),
    INTEGER(i64),
    FLOAT(f64),
    BOOLEAN(bool),
    STRING(String),
}
