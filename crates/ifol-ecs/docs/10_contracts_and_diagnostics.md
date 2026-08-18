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

## 2. Validation policy

Core luôn kiểm tra các invariant cần thiết ở registration, compile và execution.
Không có chế độ `Off` làm thay đổi semantics hoặc bỏ qua kiểm tra an toàn; host
chỉ được chạy một schedule đã compile hợp lệ.

## 3. RunReport

~~~text
RunReport
├── execution_revision
├── compiled_graph_revision
├── phases_visited
├── systems_executed
├── systems_skipped + reason
├── commands_processed
├── system errors
├── structural/data revision summary
└── optional timing/diagnostics
~~~

Timing không phải semantic correctness.

## 4. Panic policy

Input invalid, cycle, stale handle và missing data phải trả error/skip theo contract.
unwrap chỉ dùng cho invariant nội bộ đã chứng minh hoặc test.
