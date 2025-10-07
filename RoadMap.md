# RIG-Ext

RIG 是调研若干个集成相关 agent 中，function 阶段做过最多工作的，令人受益良多。目前 rmcp 虽然以及发布，但各大模型厂商是否跟进依然是一个问题，包括 hf、ollama、ds、openai 是否会按照 anthpic 的支持进行开发依然是未知数。且 anthpic 不开源。调研之后该局改造 rig 令其拥有整合 mcp 的能力，令其可以本地就可接入是一个很好的状态。

## stpe 1 mcp 整合探讨 pulgins 模式整合 sdio 模式。

### 引入 rcmp

```toml
rmcp = { version = "0.2.0", features = ["server"] }
## or dev channel
rmcp = { git = "https://github.com/modelcontextprotocol/rust-sdk", branch = "main" }
```

### stdio 整合

通过对 reqwest 的整合本质上 sse streamHttp 均属于简单的范畴，未有 stdio 是有难度。

### mcp 改造

<details>
<summary>tools合规</summary>

> 1.  默认 mcp 未来会大流行，且这是有概率的，这样就不用给每个平台或者模型搞一个 tools 必须进行的事情。
> 2.  虽然现在有人在所 format 功能，以适应所有的格式，但功能的添加尤其是 annotation param 的支持 mcp 是走在前面的。

</details>

<details>
<summary>prompt+resouce+root+tools</summary>
</details>

### rig 功能性保留

<details>
<summary>agent整合</summary>
</details>

<details>
<summary>pipline</summary>
</details>

<details>
<summary>动态架构</summary>
</details>
