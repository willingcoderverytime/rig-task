//! Example usage of the entities with SeaORM
//!
//! This example demonstrates how to use the entities to interact with the database.

use sea_orm::*;
use crate::entities::{workflow, task, plan, tool_log};



/// Create a new task entry
pub async fn create_task(db: &DatabaseConnection) -> Result<task::Model, DbErr> {
    let task = task::ActiveModel {
        ..Default::default()
    };
    
    task::Entity::insert(task).exec_with_returning(db).await
}

/// Create a new plan entry
pub async fn create_plan(db: &DatabaseConnection) -> Result<plan::Model, DbErr> {
    let plan = plan::ActiveModel {
        ..Default::default()
    };
    
    plan::Entity::insert(plan).exec_with_returning(db).await
}

/// Create a new tool log entry
pub async fn create_tool_log(db: &DatabaseConnection) -> Result<tool_log::Model, DbErr> {
    let tool_log = tool_log::ActiveModel {
        ..Default::default()
    };
    
    tool_log::Entity::insert(tool_log).exec_with_returning(db).await
}

/// Get all workflows
pub async fn get_all_workflows(db: &DatabaseConnection) -> Result<Vec<workflow::Model>, DbErr> {
    workflow::Entity::find().all(db).await
}

/// Get all tasks for a specific workflow
pub async fn get_tasks_by_workflow(db: &DatabaseConnection, workflow_id: i32) -> Result<Vec<task::Model>, DbErr> {
    task::Entity::find()
        .filter(task::Column::Wid.eq(workflow_id))
        .all(db)
        .await
}