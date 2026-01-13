fn main() {}

#[cfg(test)]
mod tests {
    use rrdb::config::launch_config::LaunchConfig;
    use rrdb::engine::DBEngine;
    use rrdb::engine::ast::dml::insert::InsertQuery;
    use rrdb::engine::ast::dml::parts::insert_values::InsertValue;
    use rrdb::engine::ast::dml::parts::select_item::SelectWildCard;
    use rrdb::engine::ast::dml::select::SelectQuery;
    use rrdb::engine::ast::types::{Column, DataType, SQLExpression, TableName};
    use rrdb::engine::encoder::schema_encoder::StorageEncoder;
    use rrdb::engine::schema::table::TableSchema;
    use rrdb::engine::types::ExecuteField;
    use tokio::fs;
    use uuid::Uuid;

    #[tokio::test]
    async fn insert_then_select_uses_table_heap() {
        let base_path = std::env::temp_dir().join(format!("rrdb_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&base_path)
            .await
            .expect("temp dir create failed");

        let mut config = LaunchConfig::default();
        config.data_directory = base_path.to_string_lossy().to_string();
        let engine = DBEngine::new(config);

        let database_name = "test_db";
        let table_name = "test_table";
        let table = TableName::new(Some(database_name.to_string()), table_name.to_string());

        let table_path = base_path
            .join(database_name)
            .join("tables")
            .join(table_name);
        fs::create_dir_all(&table_path)
            .await
            .expect("table dir create failed");

        let table_schema = TableSchema {
            table: table.clone(),
            columns: vec![
                Column::builder()
                    .set_name("id".into())
                    .set_data_type(DataType::Int)
                    .set_not_null(true)
                    .build(),
            ],
            primary_key: vec![],
            foreign_keys: vec![],
            unique_keys: vec![],
        };

        let encoder = StorageEncoder::new();
        let config_path = table_path.join("table.config");
        fs::write(&config_path, encoder.encode(table_schema))
            .await
            .expect("table config write failed");

        let insert_query = InsertQuery::builder()
            .set_into_table(table.clone())
            .set_columns(vec!["id".into()])
            .set_values(vec![InsertValue {
                list: vec![Some(SQLExpression::Integer(1))],
            }])
            .build();

        engine.insert(insert_query).await.expect("insert failed");

        let select_query = SelectQuery::builder()
            .add_select_wildcard(SelectWildCard { alias: None })
            .set_from_table(table.clone())
            .build();

        let result = engine.select(select_query).await.expect("select failed");
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(1));

        std::fs::remove_dir_all(&base_path).expect("temp dir cleanup failed");
    }
}
