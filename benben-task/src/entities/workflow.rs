use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "workflow")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String,
    pub code: Option<String>, // New plan field
    pub name: Option<String>, // New plan field
    pub desc: Option<String>, // New plan field
    pub plan: Option<String>, // New plan field
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}