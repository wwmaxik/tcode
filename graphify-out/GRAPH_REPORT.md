# Graph Report - tcode  (2026-05-14)

## Corpus Check
- 17 files · ~12,375 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 205 nodes · 326 edges · 21 communities (19 shown, 2 thin omitted)
- Extraction: 90% EXTRACTED · 10% INFERRED · 0% AMBIGUOUS · INFERRED: 32 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Community 0|Community 0]]
- [[_COMMUNITY_Community 1|Community 1]]
- [[_COMMUNITY_Community 2|Community 2]]
- [[_COMMUNITY_Community 3|Community 3]]
- [[_COMMUNITY_Community 4|Community 4]]
- [[_COMMUNITY_Community 5|Community 5]]
- [[_COMMUNITY_Community 6|Community 6]]
- [[_COMMUNITY_Community 7|Community 7]]
- [[_COMMUNITY_Community 9|Community 9]]
- [[_COMMUNITY_Community 10|Community 10]]
- [[_COMMUNITY_Community 12|Community 12]]

## God Nodes (most connected - your core abstractions)
1. `App` - 67 edges
2. `draw()` - 18 edges
3. `LspClient` - 10 edges
4. `PluginEngine` - 9 edges
5. `tcode` - 7 edges
6. `TerminalState` - 6 edges
7. `draw_explorer()` - 5 edges
8. `draw_editor()` - 5 edges
9. `main()` - 5 edges
10. `Config` - 5 edges

## Surprising Connections (you probably didn't know these)
- `main()` --calls--> `draw()`  [INFERRED]
  src/main.rs → src/ui.rs
- `main()` --calls--> `spawn_pty()`  [INFERRED]
  src/main.rs → src/term/pty.rs
- `main()` --calls--> `handle_events()`  [INFERRED]
  src/main.rs → src/events.rs

## Communities (21 total, 2 thin omitted)

### Community 1 - "Community 1"
Cohesion: 0.25
Nodes (19): draw(), draw_activity_bar(), draw_editor(), draw_explorer(), draw_folder_prompt(), draw_fuzzy_finder(), draw_generic_prompt(), draw_git_panel() (+11 more)

### Community 2 - "Community 2"
Cohesion: 0.15
Nodes (7): FileEntry, Focus, GitChange, InputMode, read_directory(), SidebarPanel, Tab

### Community 3 - "Community 3"
Cohesion: 0.17
Nodes (15): ActivityBarRect, ExplorerRect, handle_events(), handle_key(), handle_mouse(), handle_prompt_key(), handle_tab_bar_click(), handle_theme_picker_key() (+7 more)

### Community 4 - "Community 4"
Cohesion: 0.17
Nodes (5): Config, Session, Pty, PtyCommand, spawn_pty()

### Community 5 - "Community 5"
Cohesion: 0.13
Nodes (14): Build from Source, code:bash (git clone https://github.com/yourusername/tcode.git), code:rust (fn on_load() {), Editor & Selection, Features, General, Installation, Keyboard Shortcuts (+6 more)

### Community 6 - "Community 6"
Cohesion: 0.21
Nodes (4): JsonRpcNotification, JsonRpcRequest, LspClient, LspMessage

### Community 7 - "Community 7"
Cohesion: 0.23
Nodes (4): EditorApi, LoadedPlugin, PluginCommand, PluginEngine

### Community 9 - "Community 9"
Cohesion: 0.29
Nodes (3): get_project_data(), TermCell, TerminalState

### Community 10 - "Community 10"
Cohesion: 0.25
Nodes (7): bckgDimensions, elem, Graph, infoPanel, nodeName, nodePath, nodeType

## Knowledge Gaps
- **37 isolated node(s):** `TextAreaRect`, `ExplorerRect`, `TabBarRect`, `ModalRect`, `TerminalRect` (+32 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **2 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `App` connect `Community 0` to `Community 2`, `Community 4`, `Community 8`, `Community 11`, `Community 12`, `Community 13`, `Community 14`, `Community 15`, `Community 16`?**
  _High betweenness centrality (0.424) - this node is a cross-community bridge._
- **Why does `Config` connect `Community 4` to `Community 1`?**
  _High betweenness centrality (0.128) - this node is a cross-community bridge._
- **Why does `main()` connect `Community 3` to `Community 1`, `Community 4`?**
  _High betweenness centrality (0.112) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `draw()` (e.g. with `.default()` and `main()`) actually correct?**
  _`draw()` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `TextAreaRect`, `ExplorerRect`, `TabBarRect` to the rest of the system?**
  _37 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.06 - nodes in this community are weakly interconnected._
- **Should `Community 5` be split into smaller, more focused modules?**
  _Cohesion score 0.13 - nodes in this community are weakly interconnected._