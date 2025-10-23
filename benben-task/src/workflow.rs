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
pub async fn start_task(_task: TaskVo) {
    // In a real implementation, this would:
    // 1. Query the workflow by workflowid
    // 2. Create a new task in the database
    // 3. Initialize the task with the engine
    // 4. Start the task execution
    
    // For now, we're just providing the function structure
    println!("Task start functionality would be implemented here");
}

///[stop_task] 根据任务Id进行任务暂停任务执行，
/// 根据任务Id 调用 engine 完成任务task
pub async fn stop_task(task_id: &str) {
    // Parse the task_id string to i32
    match task_id.parse::<i32>() {
        Ok(id) => {
            // Get the global task engine instance
            if let Some(engine) = crate::engine::TaskEngine::global() {
                // Call the stop method on the engine
                match engine.stop(id).await {
                    Ok(_) => {
                        // Task successfully stopped
                        println!("Task {} successfully stopped", id);
                    }
                    Err(e) => {
                        // Handle error when stopping task
                        eprintln!("Failed to stop task {}: {}", id, e);
                    }
                }
            } else {
                eprintln!("Task engine not initialized");
            }
        }
        Err(_) => {
            eprintln!("Invalid task ID: {}", task_id);
        }
    }
}

/// [resume_task] 根据任务Id恢复任务执行
/// 根据任务Id调用engine完成任务恢复
pub async fn resume_task(task_id: &str) {
    // Parse the task_id string to i32
    match task_id.parse::<i32>() {
        Ok(id) => {
            // Get the global task engine instance
            if let Some(engine) = crate::engine::TaskEngine::global() {
                // Call the resume method on the engine
                match engine.resume(id).await {
                    Ok(_) => {
                        // Task successfully resumed
                        println!("Task {} successfully resumed", id);
                    }
                    Err(e) => {
                        // Handle error when resuming task
                        eprintln!("Failed to resume task {}: {}", id, e);
                    }
                }
            } else {
                eprintln!("Task engine not initialized");
            }
        }
        Err(_) => {
            eprintln!("Invalid task ID: {}", task_id);
        }
    }
}

/// [cancel_task] 根据任务Id取消任务执行
/// 根据任务Id调用engine完成任务取消
pub async fn cancel_task(task_id: &str) {
    // Parse the task_id string to i32
    match task_id.parse::<i32>() {
        Ok(id) => {
            // Get the global task engine instance
            if let Some(engine) = crate::engine::TaskEngine::global() {
                // Call the cancel method on the engine
                match engine.cancel(id).await {
                    Ok(_) => {
                        // Task successfully cancelled
                        println!("Task {} successfully cancelled", id);
                    }
                    Err(e) => {
                        // Handle error when cancelling task
                        eprintln!("Failed to cancel task {}: {}", id, e);
                    }
                }
            } else {
                eprintln!("Task engine not initialized");
            }
        }
        Err(_) => {
            eprintln!("Invalid task ID: {}", task_id);
        }
    }
}

/// [finish_task] 根据任务Id完成任务执行
/// 根据任务Id调用engine完成任务结束
pub async fn finish_task(task_id: &str) {
    // Parse the task_id string to i32
    match task_id.parse::<i32>() {
        Ok(id) => {
            // Get the global task engine instance
            if let Some(engine) = crate::engine::TaskEngine::global() {
                // Call the finish method on the engine
                match engine.finish(id).await {
                    Ok(_) => {
                        // Task successfully finished
                        println!("Task {} successfully finished", id);
                    }
                    Err(e) => {
                        // Handle error when finishing task
                        eprintln!("Failed to finish task {}: {}", id, e);
                    }
                }
            } else {
                eprintln!("Task engine not initialized");
            }
        }
        Err(_) => {
            eprintln!("Invalid task ID: {}", task_id);
        }
    }
}