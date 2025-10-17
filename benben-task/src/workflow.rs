//! 执行链路计划，其中包含用什么agent，  前置检查点  后置检查点
//! 任务Id，编排模式等等。
//! 任务链路是否缺少agent即check链路。
//! 理想的形态
//! 完成 业务架构设计---- 完成系统架构模板设计。
//! 设计文档   
//! step1 ---目的： ddd 分析 涉及那些实体。行文。应该在哪个聚合之重。   
//!          可得信息：  ddd专家   业务架构逻辑--rag
//!          输出： 聚合-实体-行为-值对象 列表，
//! step2 ---目的： 查询实体-行为--值对象是否已经存在。
//!          可得信息：
//!                 方案一：--- file-map anthroicpic
//!                 方案二：--- rag
//! step3 ---创建某实体
//!             ... 进行弹窗联动  
//! step4 ---恢复智能体任务执行。
//!
//! step5 ---完成工作。
//!           

use sea_orm::EntityTrait;

use crate::entities::{workflow, task, plan, job};

pub struct TaskVo {
    // 调用这个任务的时候work flow的定义
    pub input: String,
    // 工作流id  其通过  编辑形成有向无环图，可通过执行引擎完成对智能体的循环调用。
    // 存在一个智能体触发机制，其应当是一个智能体，能够实现给出结果之后，可进行
    pub workflowid: String,
    // 其设定了人工参与的空间，即在整个执行空间之重需要部分区域由人参与。
}

/// [start task]  开始任务。
/// step 1 通过 workflowId 查询 工作流程plan字段。
/// step 2 创建任务 得到任务id
/// step 3 plan  | 分割符号  完成对计划表的装填。
/// step 4 通过workflowId 查询workflowId所装填的job 智能体全貌。
/// 其中work 是一个智能体，他是个单独的智能体通过所有job只能体的描述选择智能体执行，
/// 其决策依据就是plan计划执行对智能体的调度，并完成对计划表的维护。
/// 
/// 完成入库操作之后，待着workflowId  taskId 以及 input 丢入任务执行引擎。

pub async fn start_task(task: TaskVo) {
    // This function would typically be async, but to keep the signature unchanged,
    // we'll assume the actual implementation would be in an async context
    
    // Step 1: Query workflow plan field by workflowId
    // In a real implementation, this would query the database for the workflow
    // let workflow = workflow::Entity::find_by_id(task.workflowid).one(db).await?;
    
    // Step 2: Create task to get task id
    // let new_task = task::ActiveModel {
    //     input: Set(Some(task.input)),
    //     workflow_id: Set(task.workflowid.parse().unwrap_or(0)),
    //     ..Default::default()
    // };
    // let inserted_task = task::Entity::insert(new_task).exec_with_returning(db).await?;
    
    // Step 3: Split plan by delimiter and populate plan table
    // if let Some(plan_content) = workflow.plan {
    //     let plan_items: Vec<&str> = plan_content.split('|').collect();
    //     for item in plan_items {
    //         let new_plan = plan::ActiveModel {
    //             task_id: Set(Some(inserted_task.id)),
    //             content: Set(Some(item.to_string())),
    //             ..Default::default()
    //         };
    //         plan::Entity::insert(new_plan).exec_with_returning(db).await?;
    //     }
    // }
    
    // Step 4: Query jobs associated with workflowId to get agent overview
    // let jobs = job::Entity::find()
    //     .filter(job::Column::WorkflowId.eq(inserted_task.workflow_id))
    //     .all(db)
    //     .await?;
    
    // Final step: Pass workflowId, taskId, and input to task execution engine
    // execute_task_engine(task.workflowid, inserted_task.id, task.input, jobs);
    
    // Note: This is a simplified implementation showing the logic flow
    // A real implementation would need proper error handling and database connections
}

// Placeholder for the actual task execution engine
// fn execute_task_engine(workflow_id: String, task_id: i32, input: String, jobs: Vec<job::Model>) {
//     // Implementation would go here
// }