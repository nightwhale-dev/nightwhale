# NightWhale 夜鲲

> dsh 生态的 **idea 交易所**。
> 同一个问题，不同的人喜欢不同的答案——把所有候选解法并列摊开，你自己选、自己 buy、自己改。

不做排行榜，不替你做决定。NightWhale 的第一原则：**让多元想法可见**，而不是把它们压成一个"最优推荐"。

## 为什么做这个

真正稀缺的不是代码——token 极大丰富的时代，代码是被解放的生产力。稀缺的是**不一样的想法**。

现在的插件市场默认"存在一个最优插件，帮你找到它"。NightWhale 反过来：同一个问题（比如"agent 跨 session 记不住偏好"）可以有多种正交解法（摘要压缩 / 结构化笔记 / 向量召回），它们是平级的候选。哪个匹配你当下的问题理解，只有你知道。

核心动作：**检索** 问题 → 看到全部候选 idea → **buy** 一个到本地（连源码一起，不是黑盒）→ 用 → 好就 **improve**，不好就 **uninstall**（卸载是有价值的淘汰信号）→ 有了新想法就 **propose** 回公共库。

## 安装

即将上线（包名已占位）：

```bash
npm install -g nightwhale     # coming soon
pip install nightwhale        # coming soon
cargo install nightwhale      # coming soon
```

## 用法

```bash
nightwhale sync                          # 同步公共 idea 库
nightwhale search "跨 session 记忆"       # 检索问题，看到并列的候选方案
nightwhale buy <problem>/<idea>          # 把某个方案的源码拉到本地
nightwhale improve <id> --note "..."     # 记录你的改进
nightwhale uninstall <id> --reason "..." # 淘汰它（留下信号）
nightwhale list                          # 你的 idea 账本
nightwhale propose --problem <p> --idea <i>  # 贡献新 idea（生成 PR 模板）
```

## 架构（MVP）

- **个人仓库** = 本地 `~/.nightwhale/`（ledger + 拉取的源码），一个 CLI 操作它，无需账户。
- **公共 idea 库** = [`nightwhale-dev/registry`](https://github.com/nightwhale-dev/registry)，GitHub repo 当数据库，提交新 idea = 发 PR。

详见 [PRD.md](./PRD.md)。

## 现状

Developer preview。核心 CLI 流程已跑通，registry 有种子数据。想跟进就点个 Star。

## License

MIT
