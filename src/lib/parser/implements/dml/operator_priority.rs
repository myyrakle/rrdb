pub fn operator_priority_map(operator: &str) -> i32 {
    match operator {
        "+" => 10,
        "-" => 10,
        "*" => 20,
        "/" => 20,
        "%" => 20,
    }
}
