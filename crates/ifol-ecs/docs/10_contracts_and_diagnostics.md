# Contract, validation và diagnostics

## 1. Validate trước execute

~~~mermaid
flowchart LR
    Input["Registration + World state"] --> Validate["Structured validation"]
    Validate -->|"invalid"| Error["Typed EcsError"]
    Validate -->|"valid"| Compile["Compiled plan"]
    Compile --> Execute["run_once"]
    Execute --> Report["RunReport"]
~~~

ECS không âm thầm bỏ qua system binding, phase dependency, stale entity hoặc
required world component mà không ghi diagnostics.

## 2. Validation levels

~~~text
Strict  → đầy đủ ID/access/dependency/invariant diagnostics
Basic   → handle/structural checks
Off     → chỉ dành cho plan đã compile và input đã tin cậy
~~~

Public/default path nên là Strict hoặc Basic có contract rõ. Validation level không
được thay đổi semantic execution.

## 3. RunReport

~~~text
RunReport
├── execution_revision
├── compiled_graph_revision
├── phases_visited
├── systems_executed
├── systems_skipped + reason
├── command outcomes
├── system errors
├── structural/data revision summary
└── optional timing/diagnostics
~~~

Timing không phải semantic correctness.

## 4. Panic policy

Input invalid, cycle, stale handle và missing data phải trả error/skip theo contract.
unwrap chỉ dùng cho invariant nội bộ đã chứng minh hoặc test.
