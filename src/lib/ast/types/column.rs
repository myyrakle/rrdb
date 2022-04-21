use crate::lib::DataType;

#[derive(Clone, Debug)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub comment: String,
    pub default: Option<String>,
    pub not_null: bool,
    pub primary_key: bool,
}

impl Column {
    pub fn builder() -> ColumnBuilder {
        ColumnBuilder::default()
    }
}

#[derive(Default)]
pub struct ColumnBuilder {
    name: Option<String>,
    data_type: Option<DataType>,
    comment: Option<String>,
    default: Option<String>,
    not_null: Option<bool>,
    primary_key: Option<bool>,
}

impl ColumnBuilder {
    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.name = Some(name);
        self
    }

    pub fn set_data_type(&mut self, data_type: DataType) -> &mut Self {
        self.data_type = Some(data_type);
        self
    }

    pub fn set_comment(&mut self, comment: String) -> &mut Self {
        self.comment = Some(comment);
        self
    }

    pub fn set_default(&mut self, default: String) -> &mut Self {
        self.default = Some(default);
        self
    }

    pub fn set_not_null(&mut self, not_null: bool) -> &mut Self {
        self.not_null = Some(not_null);
        self
    }

    pub fn set_primary_key(&mut self, primary_key: bool) -> &mut Self {
        self.primary_key = Some(primary_key);
        self
    }

    pub fn build(self) -> Column {
        Column {
            name: self.name.unwrap(),
            data_type: self.data_type.unwrap(),
            comment: self.comment.unwrap_or("".into()),
            default: self.default,
            not_null: self.not_null.unwrap_or(false),
            primary_key: self.primary_key.unwrap_or(false),
        }
    }
}
