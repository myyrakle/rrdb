#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitOffsetPlan {
    limit: Option<u32>,
    offset: Option<u32>,
}
