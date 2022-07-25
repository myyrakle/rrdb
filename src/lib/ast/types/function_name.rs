// 함수명을 가리키는 값입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionName {
    pub database_name: Option<String>,
    pub function_name: String,
}

impl FunctionName {
    pub fn is_stored_function(&self) -> bool {
        // TODO: 내장함수 처리 고도화 필요
        ["SUM", "COUNT"].contains(&self.function_name.to_uppercase().as_str())
    }
}
