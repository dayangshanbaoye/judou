-- 05 · 句读 (Judou) 初始数据库 Schema  (SQLite)
-- migration: 0001_init.sql
-- 约定：时间存 UTC ISO8601 文本；枚举用 CHECK 约束；*_json 存 JSON 文本。
-- 分词说明：
--   · 纯英文句子用 unicode61+porter（词干，'run' 可命中 'running'）——已验证 OK。
--   · 含中文的表用 trigram 作为「零依赖」默认方案。注意 trigram 限制：查询词 **必须 ≥3 字符**，
--     因此 2 字中文词（如「土豆」）无法命中。这是 SQLite 内置分词的固有限制。
--   · 中文检索若要做好（支持任意 1~2 字词、按词切分），推荐打包 FTS5 的 `simple` 扩展
--     （wangfenjin/simple，jieba 分词 + 拼音，.dll 随 Tauri 分发）。届时把下方含中文的
--     FTS 表 tokenize 由 'trigram' 改为 'simple' 即可。列为 Phase 7/8 的可选增强项。

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- ───────────────────────────── 元 / 版本 ─────────────────────────────
CREATE TABLE schema_migrations (
  version     INTEGER PRIMARY KEY,
  applied_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ───────────────────────────── 书与结构 ─────────────────────────────
CREATE TABLE books (
  id            INTEGER PRIMARY KEY,
  title         TEXT NOT NULL,
  author        TEXT,
  language      TEXT,
  file_hash     TEXT NOT NULL UNIQUE,         -- 去重
  cover_path    TEXT,
  metadata_json TEXT,
  imported_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 目录节点：自引用树（章/节/小节）
CREATE TABLE toc_nodes (
  id           INTEGER PRIMARY KEY,
  book_id      INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
  parent_id    INTEGER REFERENCES toc_nodes(id) ON DELETE CASCADE,
  title        TEXT NOT NULL,
  level        INTEGER NOT NULL DEFAULT 1,
  order_index  INTEGER NOT NULL,
  spine_href   TEXT,
  nav_anchor   TEXT,
  content_type TEXT NOT NULL DEFAULT 'body'
               CHECK (content_type IN ('introduction','preface','body','title_only','excluded')),
  included     INTEGER NOT NULL DEFAULT 1      -- 0/1 是否入库断句
);

CREATE TABLE paragraphs (
  id           INTEGER PRIMARY KEY,
  book_id      INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
  toc_node_id  INTEGER NOT NULL REFERENCES toc_nodes(id) ON DELETE CASCADE,
  order_index  INTEGER NOT NULL,
  source_href  TEXT,                           -- 来源 XHTML
  source_path  TEXT,                           -- 元素路径(追溯)
  clean_text   TEXT NOT NULL,
  raw_html     TEXT
);

-- 句子 = entry = 最小学习单元
CREATE TABLE sentences (
  id                  INTEGER PRIMARY KEY,
  book_id             INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
  paragraph_id        INTEGER NOT NULL REFERENCES paragraphs(id) ON DELETE CASCADE,
  order_index         INTEGER NOT NULL,
  text                TEXT NOT NULL,
  normalized_text     TEXT,
  start_offset        INTEGER,
  end_offset          INTEGER,
  segmentation_method TEXT NOT NULL DEFAULT 'rule'
                      CHECK (segmentation_method IN ('rule','llm','manual')),
  status              TEXT NOT NULL DEFAULT 'unread'
                      CHECK (status IN ('unread','read','understood','flagged')),
  created_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ───────────────────────────── 台账 / 规则库 ─────────────────────────────
CREATE TABLE processing_rules (
  id        INTEGER PRIMARY KEY,
  name      TEXT NOT NULL,
  stage     TEXT NOT NULL CHECK (stage IN ('classify','clean','segment','other')),
  pattern   TEXT,
  action    TEXT,
  enabled   INTEGER NOT NULL DEFAULT 1,
  version   INTEGER NOT NULL DEFAULT 1,
  notes     TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE processing_log (
  id           INTEGER PRIMARY KEY,
  book_id      INTEGER REFERENCES books(id) ON DELETE CASCADE,
  stage        TEXT NOT NULL CHECK (stage IN ('classify','clean','segment','other')),
  severity     TEXT NOT NULL DEFAULT 'info' CHECK (severity IN ('info','warn','error')),
  location_ref TEXT,                           -- href+path/offset，可跳回原文
  raw_snippet  TEXT,
  action_taken TEXT,
  source       TEXT NOT NULL DEFAULT 'rule' CHECK (source IN ('rule','llm','manual')),
  rule_id      INTEGER REFERENCES processing_rules(id) ON DELETE SET NULL,
  resolved     INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ───────────────────────────── 卡片 / 讲解 / 学习点 ─────────────────────────────
CREATE TABLE cards (
  id         INTEGER PRIMARY KEY,
  book_id    INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
  note       TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 卡片↔句子 多对多（支持多句选区）
CREATE TABLE card_sentences (
  card_id     INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
  sentence_id INTEGER NOT NULL REFERENCES sentences(id) ON DELETE CASCADE,
  order_index INTEGER NOT NULL,
  PRIMARY KEY (card_id, sentence_id)
);

-- LLM 固定四段讲解（允许多版本）
CREATE TABLE explanations (
  id             INTEGER PRIMARY KEY,
  card_id        INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
  model          TEXT,
  prompt_version TEXT,
  grammar_json   TEXT,                          -- 语法(结构化)
  translation    TEXT,                          -- 整句翻译
  plain_rephrase TEXT,                          -- 通俗表达
  raw_json       TEXT,                          -- 原始完整返回(留底)
  created_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 学习点：词/短语/句型
CREATE TABLE learning_points (
  id                    INTEGER PRIMARY KEY,
  card_id               INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
  source_explanation_id INTEGER REFERENCES explanations(id) ON DELETE SET NULL,
  type                  TEXT NOT NULL CHECK (type IN ('word','phrase','grammar','collocation','pattern')),
  surface               TEXT NOT NULL,
  lemma                 TEXT,
  definition            TEXT,
  note                  TEXT,
  promoted              INTEGER NOT NULL DEFAULT 0,   -- 是否升级(造句/复习)，需用户显式
  created_at            TEXT NOT NULL DEFAULT (datetime('now'))
);

-- C 层：我的造句（一个学习点可多句）
CREATE TABLE created_sentences (
  id                INTEGER PRIMARY KEY,
  learning_point_id INTEGER NOT NULL REFERENCES learning_points(id) ON DELETE CASCADE,
  zh_input          TEXT NOT NULL,
  en_output         TEXT,
  model             TEXT,
  prompt_version    TEXT,
  note              TEXT,                        -- LLM 说明如何用上该点
  created_at        TEXT NOT NULL DEFAULT (datetime('now'))
);

-- TTS 产物（多态归属：句子 / 造句）
CREATE TABLE audio_assets (
  id                INTEGER PRIMARY KEY,
  owner_type        TEXT NOT NULL CHECK (owner_type IN ('sentence','created_sentence')),
  owner_id          INTEGER NOT NULL,
  provider          TEXT,
  voice             TEXT,
  file_path         TEXT NOT NULL,
  duration_ms       INTEGER,
  word_timings_json TEXT,                        -- 词级时间戳(跟读高亮)
  created_at        TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 卡内追问对话
CREATE TABLE card_threads (
  id         INTEGER PRIMARY KEY,
  card_id    INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
  role       TEXT NOT NULL CHECK (role IN ('user','assistant')),
  content    TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ───────────────────────────── 复习 (FSRS) ─────────────────────────────
CREATE TABLE decks (
  id                 INTEGER PRIMARY KEY,
  name               TEXT NOT NULL,
  desired_retention  REAL NOT NULL DEFAULT 0.9,   -- 「频次」旋钮
  daily_new_limit    INTEGER NOT NULL DEFAULT 20,
  daily_review_limit INTEGER NOT NULL DEFAULT 200
);

-- 包裹任意可复习项
CREATE TABLE srs_cards (
  id          INTEGER PRIMARY KEY,
  item_type   TEXT NOT NULL CHECK (item_type IN ('learning_point','created_sentence','sentence')),
  item_id     INTEGER NOT NULL,
  deck_id     INTEGER NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
  stability   REAL,
  difficulty  REAL,
  due         TEXT,
  state       TEXT NOT NULL DEFAULT 'new' CHECK (state IN ('new','learning','review','relearning')),
  reps        INTEGER NOT NULL DEFAULT 0,
  lapses      INTEGER NOT NULL DEFAULT 0,
  last_review TEXT,
  suspended   INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (item_type, item_id)
);

CREATE TABLE review_logs (
  id              INTEGER PRIMARY KEY,
  srs_card_id     INTEGER NOT NULL REFERENCES srs_cards(id) ON DELETE CASCADE,
  rating          TEXT NOT NULL CHECK (rating IN ('again','hard','good','easy')),
  reviewed_at     TEXT NOT NULL DEFAULT (datetime('now')),
  elapsed_days    REAL,
  scheduled_days  REAL,
  state_before_json TEXT
);

-- ───────────────────────────── 索引 ─────────────────────────────
CREATE INDEX idx_toc_book_parent      ON toc_nodes(book_id, parent_id, order_index);
CREATE INDEX idx_para_node            ON paragraphs(toc_node_id, order_index);
CREATE INDEX idx_sent_para            ON sentences(paragraph_id, order_index);
CREATE INDEX idx_sent_book            ON sentences(book_id);
CREATE INDEX idx_lp_card              ON learning_points(card_id);
CREATE INDEX idx_audio_owner          ON audio_assets(owner_type, owner_id);
CREATE INDEX idx_srs_due              ON srs_cards(due, suspended);    -- 复习队列
CREATE INDEX idx_log_book_resolved    ON processing_log(book_id, resolved);

-- ───────────────────────────── 全文检索 (FTS5) ─────────────────────────────
-- 英文句子：porter 词干 + unicode61
CREATE VIRTUAL TABLE sentences_fts USING fts5(
  text, content='sentences', content_rowid='id', tokenize='porter unicode61'
);
CREATE TRIGGER sentences_ai AFTER INSERT ON sentences BEGIN
  INSERT INTO sentences_fts(rowid, text) VALUES (new.id, new.text);
END;
CREATE TRIGGER sentences_ad AFTER DELETE ON sentences BEGIN
  INSERT INTO sentences_fts(sentences_fts, rowid, text) VALUES('delete', old.id, old.text);
END;
CREATE TRIGGER sentences_au AFTER UPDATE ON sentences BEGIN
  INSERT INTO sentences_fts(sentences_fts, rowid, text) VALUES('delete', old.id, old.text);
  INSERT INTO sentences_fts(rowid, text) VALUES (new.id, new.text);
END;

-- 含中文：trigram（零依赖，查询需 ≥3 字符；2 字中文词无法命中，见顶部说明）。explanations(译文/通俗表达)
CREATE VIRTUAL TABLE explanations_fts USING fts5(
  translation, plain_rephrase, content='explanations', content_rowid='id', tokenize='trigram'
);
CREATE TRIGGER expl_ai AFTER INSERT ON explanations BEGIN
  INSERT INTO explanations_fts(rowid, translation, plain_rephrase)
  VALUES (new.id, new.translation, new.plain_rephrase);
END;
CREATE TRIGGER expl_ad AFTER DELETE ON explanations BEGIN
  INSERT INTO explanations_fts(explanations_fts, rowid, translation, plain_rephrase)
  VALUES('delete', old.id, old.translation, old.plain_rephrase);
END;
CREATE TRIGGER expl_au AFTER UPDATE ON explanations BEGIN
  INSERT INTO explanations_fts(explanations_fts, rowid, translation, plain_rephrase)
  VALUES('delete', old.id, old.translation, old.plain_rephrase);
  INSERT INTO explanations_fts(rowid, translation, plain_rephrase)
  VALUES (new.id, new.translation, new.plain_rephrase);
END;

-- 学习点
CREATE VIRTUAL TABLE learning_points_fts USING fts5(
  surface, definition, content='learning_points', content_rowid='id', tokenize='trigram'
);
CREATE TRIGGER lp_ai AFTER INSERT ON learning_points BEGIN
  INSERT INTO learning_points_fts(rowid, surface, definition) VALUES (new.id, new.surface, new.definition);
END;
CREATE TRIGGER lp_ad AFTER DELETE ON learning_points BEGIN
  INSERT INTO learning_points_fts(learning_points_fts, rowid, surface, definition)
  VALUES('delete', old.id, old.surface, old.definition);
END;
CREATE TRIGGER lp_au AFTER UPDATE ON learning_points BEGIN
  INSERT INTO learning_points_fts(learning_points_fts, rowid, surface, definition)
  VALUES('delete', old.id, old.surface, old.definition);
  INSERT INTO learning_points_fts(rowid, surface, definition) VALUES (new.id, new.surface, new.definition);
END;

-- 造句（中英混排）
CREATE VIRTUAL TABLE created_sentences_fts USING fts5(
  zh_input, en_output, content='created_sentences', content_rowid='id', tokenize='trigram'
);
CREATE TRIGGER cs_ai AFTER INSERT ON created_sentences BEGIN
  INSERT INTO created_sentences_fts(rowid, zh_input, en_output) VALUES (new.id, new.zh_input, new.en_output);
END;
CREATE TRIGGER cs_ad AFTER DELETE ON created_sentences BEGIN
  INSERT INTO created_sentences_fts(created_sentences_fts, rowid, zh_input, en_output)
  VALUES('delete', old.id, old.zh_input, old.en_output);
END;
CREATE TRIGGER cs_au AFTER UPDATE ON created_sentences BEGIN
  INSERT INTO created_sentences_fts(created_sentences_fts, rowid, zh_input, en_output)
  VALUES('delete', old.id, old.zh_input, old.en_output);
  INSERT INTO created_sentences_fts(rowid, zh_input, en_output) VALUES (new.id, new.zh_input, new.en_output);
END;

-- 默认 deck
INSERT INTO decks(name) VALUES ('默认');
INSERT INTO schema_migrations(version) VALUES (1);
