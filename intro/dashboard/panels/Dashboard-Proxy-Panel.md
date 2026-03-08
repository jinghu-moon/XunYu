# Dashboard Proxy 面板导读

这篇文档专门拆 `dashboard-ui/src/components/ProxyPanel.vue`。

这个组件的复杂度不在表单本身，而在于它同时处理了三种不同语义：

1. **持久化配置**：`defaultUrl` / `noproxy`
2. **当前生效状态**：各工具当前代理是否开启、地址是否一致
3. **探测结果**：某个代理对一组目标的连通性与时延

所以它不是一个单纯的“代理设置表单”，而是一个 **代理编排与诊断面板**。

## 1. 数据契约与接口边界

## 1.1 数据模型

这个面板主要依赖 3 类类型：

- `ProxyConfig`
  - `defaultUrl?: string | null`
  - `noproxy?: string | null`
- `ProxyItem`
  - `tool`
  - `status`
  - `address`
- `ProxyTestItem`
  - `label`
  - `ok`
  - `ms`
  - `error`

从这三个类型就能看出它的三条主线：

- 配置层
- 状态层
- 测试层

## 1.2 API 边界

它对接的 API 也正好分成三组：

### 配置组

- `fetchProxyConfig()`
- `saveProxyConfig(cfg)`

### 状态与动作组

- `fetchProxyStatus()`
- `proxySet(url, noproxy, only?)`
- `proxyDel(only?)`

### 探测组

- `proxyTest(url, targets?, { timeoutMs, jobs })`

也就是说，面板内部已经把“保存配置”和“实际应用到工具”明确区分开了。

## 2. 状态模型

## 2.1 三类核心状态

### 当前工具状态

- `items`：各工具当前代理状态列表

### 当前配置与输入

- `cfg`：后端保存的代理配置
- `url`
- `noproxy`
- `only`
- `includeMsys2`

### 探测输入与结果

- `testTargets`
- `testTimeoutMs`
- `testJobs`
- `testResult`

再加上横切状态：

- `busy`
- 删除确认计时器相关状态

## 2.2 删除确认状态

和书签面板类似，它也用了一个 3 秒确认窗：

- `removeConfirmRemaining`
- `removeConfirmTimer`
- `armRemoveConfirm()` / `resetRemoveConfirm()` / `isRemoveConfirmArmed()`

这说明代理删除也被视为需要轻确认的动作。

## 3. 计算属性：一致性判断是这个组件的核心

`ProxyPanel.vue` 最有意思的地方，不是保存，而是它对“配置”和“现实状态”的比对。

## 3.1 配置快照派生

- `currentDefaultUrl`
- `currentNoProxy`

它们从 `cfg` 里派生，用于摘要区显示当前已保存配置。

## 3.2 生效状态派生

- `activeProxyItems`：筛出 `status === 'ON'` 且地址非空的工具
- `matchCount`：有多少激活工具的地址和 `currentDefaultUrl` 一致

## 3.3 一致性派生

- `consistencyText`
- `consistencyDetail`
- `consistencyTone`

一致性判断规则可以概括为：

- 没有 `defaultUrl`：`No defaultUrl`
- 有配置但没有任何激活工具：`No active proxies`
- 所有激活工具都与 `defaultUrl` 对齐：`Consistent`
- 否则：`Drift`

这说明它不是只展示配置值，而是在主动回答：**“你保存的代理配置和当前真实环境是否漂移了？”**

## 3.4 工具作用域派生：`onlyValue`

`onlyValue` 是另一个很关键的计算属性。

它会根据：

- 当前选择的 `only`
- `includeMsys2`

组合出真正传给后端的 `only` 参数。

逻辑是：

- 选择 `all` 且勾选 `includeMsys2`：传 `undefined`，表示真正全量
- 选择 `all` 但不勾选 `includeMsys2`：只传 `cargo,git,npm`
- 选择某个子集时：按需拼接 `,msys2`

这意味着 UI 上的“Include MSYS2”不是纯展示项，而是真正参与动作作用域计算。

## 4. 核心工作流

## 4.1 加载

`load()` 会同时做两件事：

1. `fetchProxyStatus()` 拉各工具当前状态
2. `fetchProxyConfig()` 拉持久化配置

随后再把：

- `cfg.defaultUrl` 回填到 `url`
- `cfg.noproxy` 回填到 `noproxy`

因此面板一打开，用户同时看到：

- 当前已保存的配置
- 当前真实生效的工具状态
- 可直接编辑和应用的新输入

## 4.2 保存配置：`onSaveConfig()`

这个动作只做持久化：

- 调 `saveProxyConfig({ defaultUrl, noproxy })`
- 再 `load()` 重新同步

它**不会**自动调用 `proxySet()`。这点很重要，因为它把“保存默认值”和“立即应用到工具”分开了。

## 4.3 应用代理：`onApply()`

这个动作才是真正把代理推到工具层：

- 如果 `url` 为空，直接 `alert('Proxy URL is empty.')`
- 否则调 `proxySet(url, noproxy, onlyValue)`
- 然后重新 `load()`

所以 `ProxyPanel` 里有两种不同的“提交”语义：

- `Save config`：改默认配置
- `Apply`：改当前工具状态

## 4.4 移除代理：`onRemove()`

移除动作采用轻确认模式：

- 第一次点击：进入 3 秒确认态
- 第二次点击：调用 `proxyDel(onlyValue)`
- 完成后重新 `load()`

这和书签的危险动作保护风格是一致的。

## 4.5 探测代理：`onTest()`

测试动作有自己的一条独立链路：

- 校验 `url` 非空
- 读取 `testTargets`
- 读取并规范化 `timeoutMs` / `jobs`
- 调 `proxyTest(...)`
- 把结果写进 `testResult`

它和配置保存、代理应用完全解耦，这正是“诊断面板”思路的体现。

## 5. 视图结构

## 5.1 顶部工具条

顶部只有两个动作：

- `Refresh`
- `Save config`

刷新意味着这个组件非常重视“当前状态可能已经被外部改动”的场景。

## 5.2 摘要区

摘要区有三张卡：

- 当前 `defaultUrl`
- 当前 `noproxy`
- 当前一致性状态

第三张卡最重要，因为它把底层比对逻辑直接转成用户可理解的诊断信息。

## 5.3 表单区

表单区包括：

- `Default proxy URL`
- `No proxy`
- `Apply to`
- `Include MSYS2`
- `Apply`
- `Remove`

它其实不是单一表单，而是“配置输入 + 作用域控制 + 动作按钮”的组合区。

## 5.4 探测区

探测区单独成块：

- 目标列表（CSV）
- 超时
- 并发数
- `Test`
- 测试结果表格

这块和上面的配置块分开后，语义非常清楚：

- 上面是“我要怎么配 / 怎么应用”
- 这里是“应用后效果怎么样”

## 5.5 状态卡片区

最下面用卡片展示 `items`：

- 工具名
- ON / OFF 状态
- 当前地址

所以整个面板是一个自上而下的链路：

**保存默认值 → 选择应用范围 → 下发到工具 → 探测效果 → 查看状态卡片**

## 6. 组件特征总结

`ProxyPanel.vue` 最核心的设计点有 4 个：

1. **配置与应用分离**：保存默认值不等于立刻应用。
2. **状态与配置对比**：显式检测 drift，而不是只展示表单内容。
3. **作用域可控**：`only + includeMsys2` 共同决定真正下发范围。
4. **诊断链路独立**：测试不依附于保存和应用动作。

## 7. 当前实现上的观察

这里有几个实现细节值得记住：

- 面板依赖 `load()` 把配置和状态重新对齐，所以每次动作后都会全量刷新。
- “一致性”只比较激活工具的地址和当前 `defaultUrl`，不尝试更复杂的多维判断。
- URL 为空时目前走的是 `alert(...)`，而不是 `toast`，说明这类前置校验仍保留了更原生的处理方式。

## 8. 推荐阅读顺序

如果你要继续读源码，建议这样看：

1. 先看 `ProxyConfig`、`ProxyItem`、`ProxyTestItem`
2. 再看 `load()`，建立“配置 + 状态”双源模型
3. 然后看 `consistency*` 和 `onlyValue`
4. 最后看 `onSaveConfig()`、`onApply()`、`onRemove()`、`onTest()` 四条动作链

读完后你会比较清楚：这个面板真正解决的问题不是“如何填代理地址”，而是“如何把代理配置、安全应用和状态诊断放到一张工作台里”。
