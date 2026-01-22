# Claude Code 時間追蹤工具設計

## 專案資訊

- **專案名稱**：`claude-time-tracker`
- **專案路徑**：`~/workspace/rust/claude-time-tracker`
- **語言**：Rust

---

## 概述

開發一個 Rust CLI 工具 `claude-time-tracker`，透過 Claude Code Hooks 自動追蹤每個專案的使用時間，並產生月度工作報告。

## 核心需求

- **追蹤活躍時間**：排除閒置時段（超過 10 分鐘無操作）
- **專案識別**：資料夾路徑 + Git remote URL + 設定檔對應
- **工作項識別**：Branch 名稱快照 + 可選的 regex 解析（如 Linear/Trello ID）
- **Commit 關聯**：記錄 session 期間的 commits 作為工作說明
- **報告格式**：Markdown、CSV、JSON 三種格式

---

## 架構設計

```
┌─────────────────────────────────────────────────────────┐
│                    Claude Code Hooks                     │
├─────────────────────────────────────────────────────────┤
│  SessionStart    UserPromptSubmit    Stop               │
│       │                 │              │                │
│       ▼                 ▼              ▼                │
│  ┌─────────────────────────────────────────────────┐   │
│  │           claude-time-tracker (Rust CLI)         │   │
│  │                                                  │   │
│  │  • start     → 開始追蹤 session                  │   │
│  │  • heartbeat → 記錄活動時間戳                    │   │
│  │  • stop      → 結束 session，計算時間            │   │
│  │  • report    → 產生報告                          │   │
│  └─────────────────────────────────────────────────┘   │
│                          │                              │
│                          ▼                              │
│  ┌─────────────────────────────────────────────────┐   │
│  │              SQLite Database                     │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

---

## Hook 觸發時機

| Hook | 觸發時機 | 呼叫命令 | 動作 |
|------|----------|----------|------|
| SessionStart | Claude Code 啟動 | `start --path <path>` | 建立 session，記錄專案、branch、HEAD commit |
| UserPromptSubmit | 用戶送出訊息 | `heartbeat --path <path>` | 更新最後活躍時間戳 |
| Stop | 對話結束 | `stop --path <path>` | 結束 session，計算活躍時間，收集 commits |

### 活躍時間計算邏輯

```
if (當前時間 - 上次 heartbeat) > 10 分鐘:
    視為離開，開始新的活躍區間
else:
    累加活躍時間
```

### 異常處理

- 未正常結束的 session：下次 `start` 時自動關閉，用最後 heartbeat + 10 分鐘作為結束時間

---

## 資料庫結構 (SQLite)

```sql
CREATE TABLE projects (
    id INTEGER PRIMARY KEY,
    path TEXT UNIQUE NOT NULL,
    git_remote TEXT,
    display_name TEXT,
    work_item_pattern TEXT,
    created_at TIMESTAMP
);

CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    project_id INTEGER REFERENCES projects(id),
    branch TEXT NOT NULL,
    work_item TEXT,
    start_commit TEXT,
    end_commit TEXT,
    started_at TIMESTAMP NOT NULL,
    ended_at TIMESTAMP,
    active_seconds INTEGER,
    status TEXT DEFAULT 'active'  -- active | completed | abandoned
);

CREATE TABLE heartbeats (
    id INTEGER PRIMARY KEY,
    session_id INTEGER REFERENCES sessions(id),
    timestamp TIMESTAMP NOT NULL
);

CREATE TABLE commits (
    id INTEGER PRIMARY KEY,
    session_id INTEGER REFERENCES sessions(id),
    hash TEXT NOT NULL,
    message TEXT,
    committed_at TIMESTAMP
);
```

---

## CLI 命令

```bash
# Hook 呼叫
claude-time-tracker start --path <project_path>
claude-time-tracker heartbeat --path <project_path>
claude-time-tracker stop --path <project_path>

# 使用者手動呼叫
claude-time-tracker report [--month YYYY-MM] [--project <name>] [--format md|csv|json] [--output <file>]
claude-time-tracker report --all-formats --output <basename>  # 產生 .md, .csv, .json
claude-time-tracker status                                     # 顯示當前追蹤狀態
claude-time-tracker config --init|--edit|--show
claude-time-tracker projects --list|--set-name <path> <name>
```

---

## 設定檔

### 全域設定：`~/.config/claude-time-tracker/config.toml`

```toml
[settings]
idle_timeout_minutes = 10
database_path = "~/.local/share/claude-time-tracker/data.db"

[report]
default_format = "markdown"
```

### 專案設定：`<project>/.claude-time-tracker.toml`（優先讀取）

```toml
name = "客戶 A - 電商平台"
work_item_pattern = "^(?:feature|fix|chore)/([A-Z]+-\\d+)"

[report]
include_commits = true
max_commits_per_item = 10
```

### 設定讀取順序

1. 內建預設值
2. 全域設定 `~/.config/claude-time-tracker/config.toml`
3. 專案設定 `<project>/.claude-time-tracker.toml`（覆蓋前者）

---

## 實作步驟

### Phase 1：核心基礎

1. 建立 Rust 專案，設定 Cargo.toml（依賴：clap, rusqlite, serde, chrono, git2）
2. 實作 SQLite 資料庫初始化和 migration
3. 實作設定檔讀取（全域 + 專案層級）
4. 實作 Git 操作（取得 branch、remote、HEAD、commits）

### Phase 2：追蹤功能

5. 實作 `start` 命令 — 建立 session
6. 實作 `heartbeat` 命令 — 記錄活動時間戳
7. 實作 `stop` 命令 — 結束 session，計算活躍時間，收集 commits
8. 實作異常 session 處理（abandoned sessions）

### Phase 3：報告功能

9. 實作時間統計查詢（按專案、工作項、時間範圍）
10. 實作 Markdown 報告產生
11. 實作 CSV 報告產生
12. 實作 JSON 報告產生

### Phase 4：CLI 和整合

13. 實作 `status` 命令
14. 實作 `config` 命令
15. 實作 `projects` 命令
16. 設定 Claude Code hooks

---

## 關鍵檔案

```
claude-time-tracker/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI 入口
│   ├── cli.rs               # 命令定義 (clap)
│   ├── config.rs            # 設定檔處理
│   ├── db.rs                # SQLite 操作
│   ├── git.rs               # Git 操作
│   ├── tracker.rs           # 追蹤邏輯 (start/heartbeat/stop)
│   ├── report/
│   │   ├── mod.rs
│   │   ├── markdown.rs
│   │   ├── csv.rs
│   │   └── json.rs
│   └── models.rs            # 資料結構
└── README.md
```

---

## 驗證方式

1. **單元測試**：活躍時間計算邏輯、設定檔解析、工作項 regex 解析
2. **整合測試**：完整的 start → heartbeat → stop 流程
3. **手動測試**：
   - 設定 Claude Code hooks
   - 在測試專案中執行幾次對話
   - 執行 `report` 確認輸出正確
   - 測試異常情況（直接關閉 terminal、長時間閒置）

---

## 注意事項

- 使用 `git2` crate 需要系統有 `libgit2`，或考慮用 `gix`（純 Rust 實作）
- Hooks 執行速度要快，避免拖慢 Claude Code
- 資料庫寫入要處理並發（雖然通常只有單一 instance）
