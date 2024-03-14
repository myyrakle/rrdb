use crate::ast::predule::SQLExpression;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum SelectKind {
    WildCard(SelectWildCard),
    SelectItem(SelectItem),
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct SelectWildCard {
    pub alias: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct SelectItem {
    pub item: Option<SQLExpression>, // select 요소
    pub alias: Option<String>,       // as 절이 있을 경우 alias 정보
}

impl SelectItem {
    pub fn builder() -> Self {
        Self {
            item: None,
            alias: None,
        }
    }

    pub fn set_item(mut self, item: SQLExpression) -> Self {
        self.item = Some(item);
        self
    }

    pub fn set_alias(mut self, alias: String) -> Self {
        self.alias = Some(alias);
        self
    }

    pub fn build(self) -> Self {
        self
    }
}
