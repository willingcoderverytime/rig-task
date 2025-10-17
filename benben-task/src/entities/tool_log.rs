use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tool_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub taskid: Option<i32>,
    pub planid: Option<String>,
    pub args: Option<String>,
    #[sea_orm(column_name = "ouput")] // Keeping the original typo from documentation
    pub output: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}