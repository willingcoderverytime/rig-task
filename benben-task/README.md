# benben-task
> 让整个执行过程可以形成一个人类可参与的工作流程，形成人与ai在流程中的混合共存。
## 关键功能
1. 可装填各类型的agent  并实现agent的查看以及编辑。
    ---目前允许使用配置文件进行编辑已经完成，需要特定格式。  
    ---每个agent的均需要一个，均可能存在check的过程。
    ---根据结果可能存在入库子计划的入库以及后续多agent的选中等等。
2. 任务可跳出 执行画布 完成之后又可唤醒。
    ---工作流执行过程中，旁边应当由带全行业知识库的聊天窗。
    ---弹出之后workflow应当是一个pending的装态。
    ---完成之后应当能够需要进行唤醒。唤醒时需要进行next or retry的尝试。
3. todo--工作流的并发处理，目前都是arc状态，并发可以做但是先不考虑。

## WorkFlow
当前任务实现一条简单的调用工作流水线。

```sql
workid==不唯一见证唯一性。
id
pid
code
action== todo 这个可以进行 goto 未来ai可以根据 或者loop 或者if else 
desc == 功能描述
check == 结果检查
type == 跳转--回溯--等等没想好。
```

## Task
实现每一次调用前进行入库或者更新，亦或者完成借口哦进行stop | cancel | 
```sql
id
input
output
state
wid  == 当前执行 流程节点
planid == 当前执行过程当前任务id
```
## Plan
此计划为assisant 根据现状置顶的计划以及子计划
## ToolLog
用于记录工具调用，最终的计划智能体，可以进行介入，完成ToolLog 的reverse。