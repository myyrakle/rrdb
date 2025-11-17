#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitOffsetPlan {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}
