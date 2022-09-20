#[derive(Clone, Debug, PartialEq)]
pub struct LimitOffsetPlan {
    limit: Option<u32>,
    offset: Option<u32>,
}
