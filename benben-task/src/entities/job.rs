use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "job")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub workid: String,
    pub workflow_id: i32, // Foreign key to workflow
    pub pid: Option<i32>,
    pub code: Option<String>,
    pub action: Option<String>,
    #[sea_orm(column_name = "desc")]
    pub description: Option<String>,
    pub check: Option<String>,
    #[sea_orm(column_name = "type")]
    pub r#type: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::workflow::Entity",
        from = "Column::WorkflowId",
        to = "super::workflow::Column::Id"
    )]
    Workflow,
}

impl ActiveModelBehavior for ActiveModel {}