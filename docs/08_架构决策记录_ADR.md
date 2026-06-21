# 08 · 架构决策记录（ADR）

> 记录关键决策的**背景/决定/后果**，避免日后（或 Claude Code）反复纠结已定的事。每条可被新 ADR「取代(Superseded)」，但不直接删改历史。

格式：Context（为何要决策）/ Decision（决定）/ Consequences（代价与收益）/ Status。

---

## ADR-001 · 桌面壳与语言栈：Tauri 2 + Rust + Vue3，本地优先
- **Context**：Windows 单机、读私人 EPUB、重隐私、需高性能解析与本地 DB。作者已有 Tauri+Vue 经验。
- **Decision**：Tauri 2 + Rust 后端 + Vue3/TS 前端 + 本地 SQLite。无服务端、无账号、无云同步。
- **Consequences**：(+) 包小、离线、隐私、复用经验、Rust 利于解析/SRS；(−) 跨端/协作能力弱（非目标）；e2e 测试需 tauri-driver。
- **Status**：Accepted。

## ADR-002 · 单一本地存储：SQLite + FTS5
- **Context**：实体关系明确，需事务、可追溯、保存即搜索。
- **Decision**：SQLite 作唯一存储；FTS5 做全文检索；DB 访问全部经 `repo/` 层。
- **Consequences**：(+) 零运维、事务可靠、单文件备份；(−) 中文分词受内置限制（见 ADR-009）；并发写需注意 WAL。
- **Status**：Accepted。

## ADR-003 · 断句：规则为主 + 缩写词典 + LLM 兜底 + 异常台账
- **Context**：英文断句无法纯规则做到 100%（真实语料约 47% 句点非句末）。需要可持续变健壮的机制。
- **Decision**：规则分句（pragmatic 思路）+ 缩写词典为主；低置信段落 LLM 兜底（仅给边界、禁改写）；所有异常/特殊处理入 `processing_log`，可固化为 `processing_rules`。
- **Consequences**：(+) 务实、可追溯、越用越准、与产品「健壮性」目标一致；(−) 需维护 fixtures 与规则库；LLM 兜底有成本（仅按需触发）。
- **Status**：Accepted。

## ADR-004 · 间隔复习：FSRS-6（fsrs-rs），不用手动频次/旧 SM-2
- **Context**：作者最初设想「手动设复习频次」（SuperMemo 直觉）。FSRS-6（2025 末，700M 评测）已成 SOTA，Anki 默认，同等保持率少约 20–30% 复习量，且有 Rust 实现。
- **Decision**：用 `fsrs-rs`。以「4 档自评(Again/Hard/Good/Easy) + 每 deck 目标保持率 desired_retention」替代手动频次。积累日志后可本地优化个人参数。
- **Consequences**：(+) 更省复习、同栈、可个性化；(−) 调度不是「我手动定」，需接受算法主导（用目标保持率滑杆表达偏好）；需存 `review_logs` 供训练。
- **Status**：Accepted（替代「手动频次」初案）。

## ADR-005 · LLM/TTS 供应商无关 + 结构化 JSON 输出
- **Context**：成本/可用性/地域差异大；讲解需可存储可搜索而非自由文本。
- **Decision**：`LlmProvider`/`TtsProvider` trait 抽象；适配 OpenAI 兼容/Anthropic/国内模型与多家 TTS。讲解/造句强制受 schema 约束的 JSON，解析失败留 `raw_json` 报错。
- **Consequences**：(+) 可换供应商、结构化可检索、测试可 Mock；(−) 需维护适配层与 schema 校验；个别模型 JSON 稳定性差需重试。
- **Status**：Accepted。

## ADR-006 · 卡片支持多句 + 学习点一等公民 + 显式升级入复习
- **Context**：精准理解有时需跨句；一句可含多个学习点；自动全量造句/入复习会爆炸。
- **Decision**：`cards`↔`sentences` 多对多；`learning_points` 独立成表，各自有 C（造句）入口；是否造句/入复习由 `promoted` 用户显式触发。
- **Consequences**：(+) 表达力强、复习负担可控；(−) 数据模型更复杂（junction、promoted 状态）。
- **Status**：Accepted。

## ADR-007 · 可追溯不变量：sentence → paragraph → toc_node → book
- **Context**：产品硬需求——任一句子可回溯到章/节/段。
- **Decision**：强外键链 + 段落记 source_href/元素路径；设为不变量，必须有测试守护，破坏即不合并。
- **Consequences**：(+) 导航/上下文/可信；(−) 解析阶段须严谨维护锚点。
- **Status**：Accepted。

## ADR-008 · TDD + 黄金 Fixture 语料驱动解析/断句
- **Context**：解析/断句是最易回归、最难测之处。
- **Decision**：全程红-绿-重构；`fixtures/epub/` 维护最小可复现样本 + 黄金期望（`insta` 快照）；真实书问题先加 fixture 再修。CI 不联网，LLM/TTS 走 Mock，时间注入 Clock。
- **Consequences**：(+) 回归护城河、可持续健壮；(−) 需投入维护 fixtures。
- **Status**：Accepted。

## ADR-009 · 中文全文检索：默认 trigram，`simple` 扩展为升级路径
- **Context**：FTS5 内置分词对中文不友好；trigram 查询需 ≥3 字符，2 字中文词（如「土豆」）无法命中（已实测）。主搜索对象以英文为主，中文为辅。
- **Decision**：v1 含中文表用 trigram（零依赖）；英文句子用 porter+unicode61。中文检索做好则在 Phase 7/8 打包 FTS5 `simple` 扩展（jieba+拼音）替换 tokenize。
- **Consequences**：(+) v1 零额外依赖即可用；(−) v1 中文 2 字词搜索受限，需文档说明；升级需分发 .dll。
- **Status**：Accepted（v1），simple 扩展 Proposed（后续）。

---

> 新的重大决策追加 ADR-010…；若推翻旧决策，新 ADR 标注「Supersedes ADR-00X」，旧条改 Status=Superseded。
