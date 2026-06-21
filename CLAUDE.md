# CLAUDE.md · 句读 (Judou)

> 本文件是 Claude Code 每次会话的**操作手册**。开工前必读。规则为强约束（MUST/禁止），不是建议。
> 配套文档：`01_产品手册`、`02_架构方案`、`03_开发路线图`、`04_测试策略_TDD`、`05_schema.sql`、`06_IPC接口契约`、`07_提示词模板`、`08_ADR`。

---

## 0. 一句话项目说明
把一本英文原著 EPUB 拆成可追溯的句子，逐句精读→LLM 结构化讲解→用学习点造句+TTS→FSRS 间隔复习。Windows 桌面、Tauri 2 + Rust + Vue3、**本地优先、单机**。

---

## 1. 黄金法则（不可违反）

1. **TDD 优先（红-绿-重构）**：任何生产代码之前，先写一个**会失败的测试**。顺序永远是 `failing test → 最小实现 → 重构`。禁止「先写实现再补测试」。
2. **可追溯不变量**：每个 `sentence` 必须能经 `paragraph → toc_node → book` 回溯。任何破坏这条链的改动都不允许合并。相关测试必须存在。
3. **本地优先**：不引入任何服务端、账号、云同步。仅「用户主动选中送讲解/造句/合成语音」的文本可出网到 LLM/TTS。
4. **结构化输出**：LLM 讲解/造句必须产出**受 schema 约束的 JSON**（见 `07`），解析失败要留 `raw_json` 并报错，不许吞掉。
5. **异常必登记**：解析/断句的任何异常或特殊处理，必须写入 `processing_log`。这是产品健壮性的核心机制，不是可选项。
6. **断句不得改写原文**：分句（含 LLM 兜底）只决定边界，**禁止增删/改写文本**。有对应测试守护。
7. **不自动升级学习点**：`learning_points.promoted` 与是否入复习由用户显式触发，禁止自动批量造句/入队。
8. **密钥不落明文**：API Key 走 OS 凭据库（`keyring`），禁止写入代码、数据库或备份明文。

---

## 2. 技术栈
- 壳/后端：Tauri 2.x + Rust（异步用 tokio）
- 前端：Vue 3 + TypeScript（Composition API）
- DB：SQLite（`rusqlite`/`sqlx`）+ FTS5；迁移见 `05_schema.sql`
- 解析：`zip` + `quick-xml` + `lol-html`/`scraper`
- SRS：`fsrs-rs`
- HTTP：`reqwest` + `serde_json`
- 错误：Rust 用 `thiserror`（库层）/`anyhow`（应用边界）；**库代码禁止 `unwrap()/expect()`**（测试除外）

---

## 3. 仓库结构（约定）
```
judou/
├─ CLAUDE.md                # 本文件
├─ docs/                    # 01~08 文档
├─ fixtures/epub/           # 黄金测试用最小 EPUB 语料（解析/断句 TDD 的根基）
├─ src-tauri/               # Rust 后端
│  ├─ src/
│  │  ├─ ingest/            # EPUB 解析流水线（按阶段拆分，纯函数可测）
│  │  ├─ segment/           # 断句引擎 + 缩写词典 + 规则
│  │  ├─ repo/              # 数据访问层（唯一碰 DB 的地方）
│  │  ├─ llm/               # LlmProvider trait + 适配 + MockLlm
│  │  ├─ tts/               # TtsProvider trait + 适配 + MockTts
│  │  ├─ srs/               # FSRS 封装（注入 Clock）
│  │  ├─ rules/             # 规则库 + 台账引擎
│  │  ├─ commands/          # Tauri command（薄层，调用上面模块）
│  │  └─ domain/            # 领域类型/枚举/状态机
│  ├─ migrations/           # SQL 迁移
│  └─ tests/                # 集成测试（跑 fixtures）
└─ src/                     # Vue 前端
   ├─ api/                  # 类型化 IPC 客户端（对应 06）
   ├─ views/ components/ stores/
   └─ __tests__/
```

---

## 4. TDD 工作流（每个任务都照此做）
1. 去 `03_开发路线图` 找当前阶段对应的任务与验收标准。
2. **写失败测试**：把验收标准翻译成测试（Rust `#[test]` / 集成测试 / Vitest）。先跑、确认它**红**。
3. **最小实现**：只写让测试变**绿**的最少代码。
4. **重构**：在绿灯下清理；保持测试绿。
5. 解析/断句类改动：能复现的真实书问题，**先在 `fixtures/epub/` 加一个最小可复现 EPUB + 失败测试**，再修。
6. 提交：测试与实现**同一次提交**；Conventional Commits（见 §7）。

测试命令：
```
cargo test            # Rust 单元+集成
cargo clippy -- -D warnings
pnpm test             # 前端 Vitest
pnpm tauri dev        # 本地运行
```

---

## 5. 架构边界（守住分层）
- **只有 `repo/` 能碰数据库**。command/domain/ingest 不得直接写 SQL。
- **LLM/TTS 一律走 trait 抽象**；测试用 `MockLlm`/`MockTts`，CI 内**禁止真实联网调用**。
- **ingest 各阶段尽量纯函数**（输入 bytes/DOM → 输出结构体），便于单测；副作用（写库、写台账）集中在边缘。
- **时间可注入**：所有 due/调度逻辑通过 `Clock` trait 取时间，测试用固定时钟。
- command 层只做编排，不放业务逻辑。

---

## 6. Definition of Done
- 对应验收标准的测试存在且通过；`cargo clippy -D warnings` 干净。
- 不破坏可追溯不变量与黄金法则。
- 异常路径有处理与（必要时）台账记录。
- 公共行为有文档/注释；IPC 改动同步更新 `06`。
- 无明文密钥、无新增网络出网点（除非 §1.3 允许且已在设置中可见）。

---

## 7. Git / 提交约定
- Conventional Commits：`feat: / fix: / test: / refactor: / docs: / chore:`。
- 小步提交，测试+实现同提交。
- 分支名：`phaseN/<epic>-<task>`。

---

## 8. 禁止清单（Do NOT）
- ❌ 先写实现后补测试；❌ 跳过 fixtures 直接改解析逻辑。
- ❌ 在 repo 以外写 SQL；❌ 让 command 直接发 HTTP。
- ❌ 让断句改写原文；❌ 自动把学习点造句/入复习。
- ❌ 在库代码 `unwrap()`；❌ 把密钥写进代码/库。
- ❌ 引入服务端/账号/云；❌ 为「省事」绕过结构化 JSON 校验。

---

## 9. 接到任务时的标准动作
> 「实现 X 功能」→ 先读 `03` 里 X 的验收标准 → 写失败测试 → 最小实现 → 重构 → 更新相关文档 → 提交。
> 涉及解析/断句 → 先加/选 fixture。涉及 IPC → 先改 `06` 契约再实现两端。涉及 LLM/TTS → 先定/改 `07` 提示词与 schema，再用 Mock 测。
