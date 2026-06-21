# 04 · 测试策略（TDD）

> 目标：让「先写测试」在本项目可执行、可持续。重点解决最难测的两块——**EPUB 解析/断句**与**LLM/TTS 联网**。

---

## 1. 原则
- **红-绿-重构**：先写失败测试，再写最小实现，再重构。
- **可测性是设计约束**：解析阶段纯函数化、副作用集中边缘、时间/网络可注入。
- **CI 不联网**：LLM/TTS 在测试中一律 Mock；真实联网只在本地手动、可选、显式开关下进行。
- **回归优先**：真实书踩到的每个坑，先变成一个最小 fixture + 失败测试，再修。

---

## 2. 测试金字塔（本项目形态）

```
        e2e (极少数关键路径)
      ┌────────────────────┐
      │  集成测试            │  解析流水线跑 fixtures；repo 跑临时 SQLite；
      │ (Rust integration)  │  LLM/TTS 走 Mock；SRS 走固定时钟
      ├────────────────────┤
      │  单元测试 (最多)      │  断句规则、分类器、mapper、FSRS 封装、
      │  Rust unit + Vitest │  schema 解析、repo 查询、前端组件
      └────────────────────┘
```

- **Rust 单元**：纯逻辑——断句规则、缩写判定、内容分类、explanation JSON→实体 mapper、FSRS rating→调度、可追溯查询。
- **Rust 集成**（`src-tauri/tests/`）：完整解析一个 fixture EPUB，断言 toc/段落/句子与追溯；repo 层对临时文件 SQLite 做 CRUD + FTS。
- **前端 Vitest + Vue Test Utils**：阅读器交互、卡片渲染、复习评分、状态机切换。
- **e2e（tauri-driver/WebDriver 或 Playwright）**：仅覆盖少数关键路径（导入→精读→讲解→造句→复习），数量克制。

---

## 3. 最难测的部分及策略

### 3.1 解析 / 断句 —— 黄金 Fixture 语料（核心）
- `fixtures/epub/` 维护一批**手写最小 EPUB**，每个只复现一种情况：
  - 脚注：epub3 `noteref/aside`、epub2、传统 `<sup><a href="#fnN">`
  - frontmatter/bodymatter/titlepage/copyright-page 分类
  - 缩写（Mr./e.g./U.S.）、人名首字母（D. H. Lawrence）、小数/序号
  - 引号跨句、对话、列表、图注、嵌套多级目录、title-only 节点
- 每个 fixture 配**黄金期望**（toc 树 / 段落 / 句子序列）。
- 用 `insta` 做快照断言；新书出问题→**先加最小 fixture（红）→再修（绿）**。这正是「处理台账→规则库→越用越健壮」的 TDD 形态。
- 守护测试：`所有句子拼接 == 段落原文`（断句不改写原文）。

### 3.2 LLM —— Mock Provider + Schema 校验
- `LlmProvider` trait；测试用 `MockLlm` 返回**固定结构化 JSON**。
- 断言：mapper 正确把 JSON 落成 explanation/learning_points；**非法 JSON → 报错 + 留 raw_json**，不静默吞。
- 可选 `live` 测试：`#[ignore]` 或 `env JUDOU_LIVE_LLM=1` 才跑，仅本地人工核对提示词质量，不进 CI。

### 3.3 TTS —— Mock Provider
- `TtsProvider` trait；`MockTts` 返回假音频字节 + 假词级时间戳。
- 断言：`audio_assets` 行写入、文件落地、`word_timings_json` 解析正确。CI 不真实合成。

### 3.4 时间 / 调度 —— 注入 Clock
- 所有 due/间隔逻辑经 `Clock` trait 取 now；测试用 `FixedClock`。
- FSRS：给定 rating 序列 + 固定时钟，断言 due/状态推进确定可复现。

### 3.5 数据库 / FTS
- repo 测试对**临时文件 SQLite**（非内存，确保 FTS5 行为一致）跑迁移再测。
- 覆盖：可追溯外键、级联删除（删书→句/卡/音频清理）、FTS 中英文混排命中（见 `05` 分词）。

---

## 4. 覆盖率取向（务实）
- **高覆盖**：断句/分类/mapper/repo/FSRS（领域核心）。
- **中覆盖**：command 编排、前端关键组件。
- **低覆盖可接受**：纯 UI 样式、第三方适配薄壳（但适配层要有 1 个契约测试）。
- 不追指标数字，追**关键不变量被测试守护**。

---

## 5. 测试命名与组织
- Rust：`mod tests` 内 `fn <被测>_<场景>_<期望>()`，如 `segment_handles_abbreviation_no_split()`。
- 集成：按 fixture 命名，`parse_epub3_footnotes_strips_and_preserves()`。
- 前端：`describe('SentenceCard') it('renders four sections')`。

---

## 6. CI 闸门
```
cargo test && cargo clippy -- -D warnings && pnpm test && pnpm vue-tsc --noEmit
```
全绿才允许合并/进下一阶段。e2e 可单独 job、允许较慢。

---

## 7. 不变量清单（必须有测试守护）
1. 句子可追溯到段→节点→书。
2. 断句不改写原文（拼接==原文）。
3. 非法 LLM 输出不静默吞、留 raw_json。
4. 未 promoted 的学习点不入复习队列。
5. 复习队列只含到期未暂停项且不超每日上限。
6. 删除一本书会清理其全部下游数据与音频文件。
7. 密钥不出现在 DB/导出/日志中。
