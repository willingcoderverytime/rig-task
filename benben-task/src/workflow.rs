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


pub struct TaskEngine{

    pub input:String,
    // 工作流id  其通过  编辑形成有向无环图，可通过执行引擎完成对智能体的循环调用。
    // 存在一个智能体触发机制，其应当是一个智能体，能够实现给出结果之后，可进行
    pub workflowid:String,
    // 其设定了人工参与的空间，即在整个执行空间之重需要部分区域由人参与。
}


pub struct Task {}

impl Task {
    pub fn check_in() {}

    pub fn chek_out() {}
}

pub struct Job {
    // 提示词
    pub code: String,

    // code
}

