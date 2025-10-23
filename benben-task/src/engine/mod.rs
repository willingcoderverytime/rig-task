//! 接收taskid ， input ，jobs ，desc， 完成自动化执行。
//! 
//! 1、创建一个可唤醒 可暂停 pending cancle finish 的任务管理器，waiting。
//! 2、任务是异步执行的，其依靠执行任务时的，状态管理器进行控制。
//! 3、任务的执行过程可根据任务调用ai程序进行处理。其本质作用是为了使用更少的token完成更长链路的工作。
//! 4、长趋势的留痕有助于任务的连贯性。

pub mod adapter;
pub mod runnings;


use crate::entities::{task, job, tool_log, workflow};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait};
use sea_orm::ActiveValue::Set;
use once_cell::sync::OnceCell;

/// 任务状态枚举
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Running,
    Stopped,
    Cancelled,
    Finished,
    Pending,
    Waiting,
}

impl TaskState {
    /// 将TaskState转换为字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskState::Running => "running",
            TaskState::Stopped => "stopped",
            TaskState::Cancelled => "cancelled",
            TaskState::Finished => "finished",
            TaskState::Pending => "pending",
            TaskState::Waiting => "waiting",
        }
    }
}

/// 单个任务的上下文信息
#[derive(Debug, Clone)]
pub struct TaskContext {
    /// 任务状态
    pub state: TaskState,
    /// 当前任务
    pub task: Option<task::Model>,
    /// 当前工作流
    pub workflow: Option<workflow::Model>,
    /// 任务执行历史记录
    pub execution_history: Vec<String>,
}

// Static instance for global access
static ENGINE_INSTANCE: OnceCell<Arc<TaskEngine>> = OnceCell::new();

/// 任务引擎核心结构
pub struct TaskEngine {
    /// 多个任务的上下文，以任务ID为键
    tasks: Arc<Mutex<HashMap<i32, TaskContext>>>,
    /// 数据库连接
    db: Option<Arc<DatabaseConnection>>,
}

impl TaskEngine {
    /// 创建新的任务引擎实例
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            db: None,
        }
    }

    /// 获取全局任务引擎实例
    pub fn global() -> Option<Arc<TaskEngine>> {
        ENGINE_INSTANCE.get().cloned()
    }

    /// 初始化全局任务引擎实例
    pub fn init_global(engine: TaskEngine) -> Result<Arc<TaskEngine>, Box<dyn std::error::Error>> {
        let engine = Arc::new(engine);
        ENGINE_INSTANCE.set(engine.clone()).map_err(|_| "Failed to set global engine instance")?;
        Ok(engine)
    }

    /// 设置数据库连接
    pub fn with_db(mut self, db: Arc<DatabaseConnection>) -> Self {
        self.db = Some(db);
        self
    }

    /// 初始化任务引擎，设置任务ID和输入
    pub async fn init(&mut self, task_id: i32, input: String) -> Result<(), Box<dyn std::error::Error>> {
        let mut tasks = self.tasks.lock().await;
        
        let task_context = TaskContext {
            state: TaskState::Waiting,
            task: Some(task::Model {
                id: task_id,
                input: Some(input),
                output: None,
                state: Some("waiting".to_string()),
                wid: None,
                planid: None,
            }),
            workflow: None,
            execution_history: Vec::new(),
        };
        
        tasks.insert(task_id, task_context);
        Ok(())
    }

    /// 更新数据库中的任务状态
    async fn update_task_state_in_db(&self, task_id: i32, state: TaskState) -> Result<(), Box<dyn std::error::Error>> {
        // 如果没有数据库连接，直接返回
        if let Some(ref db) = self.db {
            // 查找并更新任务状态
            let task_model = task::Entity::find_by_id(task_id).one(db.as_ref()).await?;
            
            if let Some(task_model) = task_model {
                let mut task_active_model: task::ActiveModel = task_model.into();
                task_active_model.state = Set(Some(state.as_str().to_string()));
                task_active_model.update(db.as_ref()).await?;
            }
        }
        Ok(())
    }

    /// 检查状态转换是否合法
    fn is_valid_state_transition(current_state: &TaskState, new_state: &TaskState) -> bool {
        match current_state {
            // Stopped状态不能转换为Finish或Cancel状态
            TaskState::Stopped => {
                match new_state {
                    TaskState::Finished | TaskState::Cancelled => false,
                    _ => true,
                }
            },
            _ => true, // 其他状态转换都是允许的
        }
    }

    /// 启动指定任务的执行
    pub async fn start(&self, task_id: i32) -> Result<(), Box<dyn std::error::Error>> {
        let mut tasks = self.tasks.lock().await;
        if let Some(context) = tasks.get_mut(&task_id) {
            // 检查状态转换是否合法
            if !Self::is_valid_state_transition(&context.state, &TaskState::Running) {
                return Err(format!("Cannot transition from {:?} to Running state", context.state).into());
            }
            
            context.state = TaskState::Running;
            context.execution_history.push("Task started".to_string());
            
            // 更新数据库中的状态
            drop(tasks); // 释放锁以避免死锁
            self.update_task_state_in_db(task_id, TaskState::Running).await?;
            Ok(())
        } else {
            Err("Task not found".into())
        }
    }

    /// 暂停指定任务的执行
    pub async fn pause(&self, task_id: i32) -> Result<(), Box<dyn std::error::Error>> {
        let mut tasks = self.tasks.lock().await;
        if let Some(context) = tasks.get_mut(&task_id) {
            // 检查状态转换是否合法
            if !Self::is_valid_state_transition(&context.state, &TaskState::Pending) {
                return Err(format!("Cannot transition from {:?} to Pending state", context.state).into());
            }
            
            context.state = TaskState::Pending;
            context.execution_history.push("Task paused".to_string());
            
            // 更新数据库中的状态
            drop(tasks); // 释放锁以避免死锁
            self.update_task_state_in_db(task_id, TaskState::Pending).await?;
            Ok(())
        } else {
            Err("Task not found".into())
        }
    }

    /// 恢复指定任务的执行
    pub async fn resume(&self, task_id: i32) -> Result<(), Box<dyn std::error::Error>> {
        let mut tasks = self.tasks.lock().await;
        if let Some(context) = tasks.get_mut(&task_id) {
            // 检查状态转换是否合法
            if !Self::is_valid_state_transition(&context.state, &TaskState::Running) {
                return Err(format!("Cannot transition from {:?} to Running state", context.state).into());
            }
            
            context.state = TaskState::Running;
            context.execution_history.push("Task resumed".to_string());
            
            // 更新数据库中的状态
            drop(tasks); // 释放锁以避免死锁
            self.update_task_state_in_db(task_id, TaskState::Running).await?;
            Ok(())
        } else {
            Err("Task not found".into())
        }
    }

    /// 取消指定任务的执行
    pub async fn cancel(&self, task_id: i32) -> Result<(), Box<dyn std::error::Error>> {
        let mut tasks = self.tasks.lock().await;
        if let Some(context) = tasks.get_mut(&task_id) {
            // 检查状态转换是否合法
            if !Self::is_valid_state_transition(&context.state, &TaskState::Cancelled) {
                return Err(format!("Cannot transition from {:?} to Cancelled state", context.state).into());
            }
            
            context.state = TaskState::Cancelled;
            context.execution_history.push("Task cancelled".to_string());
            
            // 更新数据库中的状态
            drop(tasks); // 释放锁以避免死锁
            self.update_task_state_in_db(task_id, TaskState::Cancelled).await?;
            Ok(())
        } else {
            Err("Task not found".into())
        }
    }

    /// 完成指定任务的执行
    pub async fn finish(&self, task_id: i32) -> Result<(), Box<dyn std::error::Error>> {
        let mut tasks = self.tasks.lock().await;
        if let Some(context) = tasks.get_mut(&task_id) {
            // 检查状态转换是否合法
            if !Self::is_valid_state_transition(&context.state, &TaskState::Finished) {
                return Err(format!("Cannot transition from {:?} to Finished state", context.state).into());
            }
            
            context.state = TaskState::Finished;
            context.execution_history.push("Task finished".to_string());
            
            // 更新数据库中的状态
            drop(tasks); // 释放锁以避免死锁
            self.update_task_state_in_db(task_id, TaskState::Finished).await?;
            Ok(())
        } else {
            Err("Task not found".into())
        }
    }

    /// 停止指定任务的执行
    pub async fn stop(&self, task_id: i32) -> Result<(), Box<dyn std::error::Error>> {
        let mut tasks = self.tasks.lock().await;
        if let Some(context) = tasks.get_mut(&task_id) {
            // 检查状态转换是否合法
            if !Self::is_valid_state_transition(&context.state, &TaskState::Stopped) {
                return Err(format!("Cannot transition from {:?} to Stopped state", context.state).into());
            }
            
            context.state = TaskState::Stopped;
            context.execution_history.push("Task stopped".to_string());
            
            // 更新数据库中的状态
            drop(tasks); // 释放锁以避免死锁
            self.update_task_state_in_db(task_id, TaskState::Stopped).await?;
            Ok(())
        } else {
            Err("Task not found".into())
        }
    }

    /// 获取指定任务的当前状态
    pub async fn get_state(&self, task_id: i32) -> Result<TaskState, Box<dyn std::error::Error>> {
        let tasks = self.tasks.lock().await;
        if let Some(context) = tasks.get(&task_id) {
            Ok(context.state.clone())
        } else {
            Err("Task not found".into())
        }
    }

    /// 获取所有任务的ID列表
    pub async fn list_tasks(&self) -> Vec<i32> {
        let tasks = self.tasks.lock().await;
        tasks.keys().cloned().collect()
    }

    /// 执行任务中的作业
    pub async fn execute_job(&self, task_id: i32, job: job::Model) -> Result<String, Box<dyn std::error::Error>> {
        let mut tasks = self.tasks.lock().await;
        if let Some(context) = tasks.get_mut(&task_id) {
            let record = format!("Executing job: {:?}", job);
            context.execution_history.push(record);
            
            // 模拟作业执行
            let result = format!("Job {} executed with action {:?}", job.id, job.action);
            
            // 记录工具调用日志
            self.log_tool_call(context, job.id, result.clone()).await?;
            
            Ok(result)
        } else {
            Err("Task not found".into())
        }
    }

    /// 记录工具调用日志
    async fn log_tool_call(&self, context: &mut TaskContext, job_id: i32, output: String) -> Result<(), Box<dyn std::error::Error>> {
        // 在实际实现中，这里应该将日志写入数据库
        let _log = tool_log::Model {
            id: 0, // This would be auto-generated in real implementation
            taskid: context.task.as_ref().map(|t| t.id),
            planid: None,
            args: None,
            output: Some(output),
        };
        
        context.execution_history.push(format!("Tool log recorded for job {}", job_id));
        Ok(())
    }

    /// 获取指定任务的执行历史
    pub async fn get_execution_history(&self, task_id: i32) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let tasks = self.tasks.lock().await;
        if let Some(context) = tasks.get(&task_id) {
            Ok(context.execution_history.clone())
        } else {
            Err("Task not found".into())
        }
    }
    
    /// 移除已完成的任务
    pub async fn remove_task(&self, task_id: i32) -> Result<(), Box<dyn std::error::Error>> {
        let mut tasks = self.tasks.lock().await;
        if tasks.remove(&task_id).is_some() {
            Ok(())
        } else {
            Err("Task not found".into())
        }
    }
}

impl Default for TaskEngine {
    fn default() -> Self {
        Self::new()
    }
}