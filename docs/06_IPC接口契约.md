# 06 · IPC 接口契约（Tauri Command / Event）

> 前后端的契约。改接口先改本文档，再实现两端。建议用 `ts-rs` 或 `specta` 从 Rust 类型生成 TS，前端 `src/api/` 维护类型化客户端。
> 约定：命令名 `snake_case`；长任务用 **事件**回传进度，不阻塞；统一错误类型。

---

## 0. 统一错误
```ts
type JudouError = {
  code: 'IO'|'PARSE'|'DB'|'LLM'|'TTS'|'VALIDATION'|'NOT_FOUND'|'NETWORK'|'UNKNOWN';
  message: string;     // 面向用户的可读信息
  detail?: string;     // 调试细节（不含密钥）
};
```
所有命令失败时 reject 一个 `JudouError`。

## 0.1 连通性 / Walking Skeleton
| Command | 参数 | 返回 | 说明 |
|---|---|---|---|
| `ping` | `{ payload: string }` | `{ message: string; job_id: string }` | Phase 0 连通性检查；返回 `pong: <payload>` |

**事件**：`ping://pong` → `{ message, job_id }`。command 返回前同步 emit，用于验证前端事件订阅链路。

---

## 1. 书架 / 导入
| Command | 参数 | 返回 | 说明 |
|---|---|---|---|
| `import_epub` | `{ path: string }` | `{ job_id: string }` | 启动异步解析；进度走事件；后端写入本地 `judou.sqlite3` |
| `list_books` | `—` | `Book[]` | 含进度统计（句数/已精读/卡片/今日待复习） |
| `get_book` | `{ book_id }` | `Book` | |
| `delete_book` | `{ book_id }` | `void` | 级联清理下游数据与音频文件 |
| `get_import_report` | `{ book_id }` | `ImportReport` | 从本地库重建导入报告；当前含 TOC/段落/句子计数，异常计数后续补齐 |
| `confirm_scope` | `{ book_id, nodes: {id, content_type, included}[] }` | `void` | 范围确认页提交；之后才执行断句落库 |

**事件**：`import://progress` → `{ job_id, stage, percent, message }`；`import://done` → `{ job_id, book_id, report }`；`import://error` → `{ job_id, error }`。

`ImportReport` 当前字段：
```ts
type ImportReport = {
  book_id:number;
  title:string;
  root_toc_nodes:number;
  toc_nodes_total:number;
  included_toc_nodes:number;
  title_only_toc_nodes:number;
  excluded_toc_nodes:number;
  chapters_imported:number;
  paragraphs_imported:number;
  sentences_imported:number;
};
```

---

## 2. 阅读 / 句子
| Command | 参数 | 返回 | 说明 |
|---|---|---|---|
| `get_reader_view` | `{ book_id, toc_node_id?: number|null }` | `ReaderView` | Phase 3 已落地；一次返回书名、扁平 TOC、当前节点面包屑、段落与句子流 |
| `update_sentence_status` | `{ sentence_id, status }` | `ReaderSentence` | Phase 3 已落地；status: unread/read/understood/flagged |
| `merge_sentences` | `{ sentence_ids: number[] }` | `ReaderSentence` | Phase 3 已落地；仅允许同段连续句子；写 `processing_log(source='manual')` |
| `split_sentence` | `{ sentence_id, split_offset }` | `ReaderSentence[]` | Phase 3 已落地；按原文 byte offset 拆分；写 `processing_log(source='manual')` |
| `get_toc` | `{ book_id }` | `TocNode[]`（树） | 含每节点待复习角标 |
| `get_sentences` | `{ toc_node_id, mode }` | `Sentence[]` | mode: 'continuous'\|'focus' |
| `get_sentence_context` | `{ sentence_id }` | `{ breadcrumb, paragraph }` | 书>章>节>段 |

`ReaderView` 当前字段：
```ts
type ReaderView = {
  book_id:number;
  book_title:string;
  active_toc_node_id:number;
  breadcrumb:{ id:number; title:string }[];
  toc_nodes:{
    id:number; parent_id:number|null; title:string; level:number;
    order_index:number; content_type:string; included:boolean;
  }[];
  paragraphs:{
    id:number; toc_node_id:number; order_index:number; source_href:string;
    sentences:ReaderSentence[];
  }[];
};

type ReaderSentence = {
  id:number;
  paragraph_id:number;
  order_index:number;
  text:string;
  status:'unread'|'read'|'understood'|'flagged';
};
```

---

## 3. 卡片 / 讲解
| Command | 参数 | 返回 | 说明 |
|---|---|---|---|
| `create_card` | `{ sentence_ids: number[] }` | `Card` | 1 或多句 → 一张卡 |
| `explain_card` | `{ card_id, prompt_version? }` | `{ job_id }` | 异步调 LLM |
| `get_card` | `{ card_id }` | `CardDetail` | 含讲解、学习点、对话 |
| `reexplain_card` | `{ card_id, model?, prompt_version? }` | `{ job_id }` | 重讲，保留历史版本 |
| `ask_in_card` | `{ card_id, question }` | `{ job_id }` | 卡内追问，流式 |

**事件**：`llm://stream` → `{ job_id, delta }`；`llm://done` → `{ job_id, card_id }`；`llm://error`。

---

## 4. 学习点 / 造句 / TTS
| Command | 参数 | 返回 | 说明 |
|---|---|---|---|
| `list_learning_points` | `{ card_id }` | `LearningPoint[]` | |
| `promote_learning_point` | `{ id, promoted }` | `void` | 显式升级闸门 |
| `create_sentence` | `{ learning_point_id, zh_input }` | `{ job_id }` | LLM 造句 |
| `list_created_sentences` | `{ learning_point_id }` | `CreatedSentence[]` | |
| `synthesize_audio` | `{ owner_type, owner_id, voice? }` | `{ job_id }` | TTS |
| `get_audio` | `{ owner_type, owner_id }` | `AudioAsset[]` | |

**事件**：`tts://done` → `{ job_id, audio_id, file_path }`；`tts://error`。

---

## 5. 复习 (SRS)
| Command | 参数 | 返回 | 说明 |
|---|---|---|---|
| `get_due_summary` | `—` | `{ due_today: number, new_today: number }` | 启动提醒 |
| `start_review` | `{ deck_id? }` | `ReviewItem[]` | 取到期队列 |
| `grade_review` | `{ srs_card_id, rating }` | `{ next_due, state }` | 4 档自评→FSRS |
| `add_to_review` | `{ item_type, item_id, deck_id? }` | `SrsCard` | 入队（需 promoted） |
| `suspend_srs` | `{ srs_card_id, suspended }` | `void` | |
| `get_review_stats` | `{ range? }` | `ReviewStats` | 完成/连续天数/到期曲线 |
| `optimize_fsrs` | `{ deck_id? }` | `{ params }` | 基于日志拟合个人参数（可选） |

---

## 6. 搜索 / 台账 / 设置
| Command | 参数 | 返回 | 说明 |
|---|---|---|---|
| `search` | `{ query, filters? }` | `SearchResult[]` | 跨实体 FTS |
| `list_processing_log` | `{ book_id?, resolved? }` | `LogEntry[]` | 台账 |
| `resolve_log` | `{ log_id, action }` | `void` | 标记/忽略 |
| `promote_log_to_rule` | `{ log_id, rule }` | `Rule` | 固化为规则 |
| `list_rules` | `{ stage? }` | `Rule[]` | |
| `toggle_rule` | `{ rule_id, enabled }` | `void` | |
| `get_settings` / `update_settings` | `Settings` | `Settings` | LLM/TTS/复习/数据 |
| `set_api_key` | `{ provider, key }` | `void` | 存 keyring，**不回显、不入库** |
| `export_data` | `{ path }` | `void` | 备份 |

---

## 7. 核心 DTO（节选，最终以 Rust 生成的 TS 为准）
```ts
type Book = { id:number; title:string; author?:string; language?:string;
  stats:{ sentences:number; understood:number; cards:number; due_today:number } };

type TocNode = { id:number; parent_id?:number; title:string; level:number;
  content_type:'introduction'|'preface'|'body'|'title_only'|'excluded';
  included:boolean; due_badge:number; children:TocNode[] };

type Sentence = { id:number; text:string; order_index:number;
  status:'unread'|'read'|'understood'|'flagged'; has_card:boolean };

type Explanation = { grammar:GrammarBlock; phrases:Phrase[];
  translation:string; plain_rephrase:string };

type LearningPoint = { id:number; type:'word'|'phrase'|'grammar'|'collocation'|'pattern';
  surface:string; definition?:string; promoted:boolean };

type ReviewItem = { srs_card_id:number; item_type:string; prompt:string;
  answer:string; audio?:AudioAsset };
```

---

## 8. 约束
- 命令层（`commands/`）**只编排**，业务逻辑在领域模块；DB 只经 `repo/`。
- 长任务一律 `{ job_id }` + 事件；前端用 job_id 关联进度。
- 任何返回**绝不含 API Key**；`set_api_key` 单向写入 keyring。
- 契约变更必须同步更新本文件与 `ts-rs` 生成，并补/改对应测试。
