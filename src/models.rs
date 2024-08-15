#[derive(sqlx::FromRow, Clone)]
pub struct TableColumn {
    pub(crate) table_name: String,
    pub(crate) column_name: String,
    pub(crate) udt_name: String,
    pub(crate) data_type: String,
    pub(crate) is_nullable: bool,
    pub(crate) is_unique: bool,
    pub(crate) is_primary_key: bool,
    pub(crate) foreign_key_table: Option<String>,
    pub(crate) foreign_key_id: Option<String>,
    // #todo
    pub(crate) table_schema: String,
}

#[derive(sqlx::FromRow, Clone)]
pub struct UserDefinedEnums {
    pub(crate) enum_name: String,
    pub(crate) enum_value: String,
}
