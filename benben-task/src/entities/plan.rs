use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "plan")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub pid: Option<i32>,
    pub state: Option<String>, // success or failure
    pub planid: Option<String>, // current execution task id
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}