┌─────────────────────────────────────────────┐
│                   App (UI)                  │
│  holds ViewState, sends Commands            │
└────────────────┬────────────────────────────┘
                 │  queries via
                 ▼
┌─────────────────────────────────────────────┐
│              LibraryService                 │
│  single owned struct, lives on App          │
│  exposes a clean query/command API          │
│  owns the redb handle + TOML meta path      │
└────────────┬──────────────┬─────────────────┘
             │              │
             ▼              ▼
┌────────────────┐  ┌───────────────────┐
│  index.redb    │  │  meta/*.toml      │
│  fast lookups  │  │  source of truth  │
└────────────────┘  └───────────────────┘
