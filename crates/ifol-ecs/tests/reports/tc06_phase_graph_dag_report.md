# Báo Cáo Kiểm Thử: TC06 - Đồ Thị Pha, Sắp Xếp Topo Kahn & Phát Hiện Chu Trình (Phase DAG)

> **Crate:** `ifol-ecs`  
> **Source Test:** [`tests/slice06_phase_graph.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/tests/slice06_phase_graph.rs)  
> **Module liên quan:** [`src/schedule/graph.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/schedule/graph.rs), [`src/registry/phase_registry.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/registry/phase_registry.rs)  
> **Trạng thái:** **ĐẠT (PASS ✅)**

---

## 1. Thông Tin Test Case
- **Mã test:** `TC06` (Slice 06)
- **Tên:** Thuật toán Kahn sắp xếp thứ tự thực thi đồ thị có hướng không chu trình (DAG) và phát hiện chu trình phụ thuộc (*Cycle Detection*)
- **Mục tiêu kiểm thử:**
  1. Đăng ký 5 Phase theo thứ tự ngẫu nhiên: `submit`, `finalize`, `simulate`, `prepare`, `graph`.
  2. Thiết lập các cạnh phụ thuộc: `prepare -> simulate -> finalize -> graph -> submit`.
  3. Kiểm tra thứ tự topo sau biên dịch đúng 100% theo chuỗi phụ thuộc.
  4. Kiểm tra phát hiện chu trình trực tiếp 2 node: `PhaseA -> PhaseB -> PhaseA` $\rightarrow$ Trả về `Err(PhaseCycleDetected)`.
  5. Kiểm tra phát hiện phụ thuộc vào Phase chưa đăng ký $\rightarrow$ Trả về `Err(PhaseNotFound)`.

---

## 2. Sơ Đồ Trực Quan Đồ Thị Phụ Thuộc & Thứ Tự Biên Dịch

```text
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│     prepare     │ ────> │    simulate     │ ────> │    finalize     │ ────> │      graph      │ ────> │     submit      │
│  (in-degree: 0) │       │  (in-degree: 1) │       │  (in-degree: 1) │       │  (in-degree: 1) │       │  (in-degree: 1) │
└─────────────────┘       └─────────────────┘       └─────────────────┘       └─────────────────┘       └─────────────────┘

📌 THỨ TỰ BIÊN DỊCH BỞI THUẬT TOÁN KAHN:
   [1] prepare ──> [2] simulate ──> [3] finalize ──> [4] graph ──> [5] submit
```

---

## 3. Kết Quả & Đánh Giá Chi Tiết

| Kịch bản kiểm tra | Đồ thị đầu vào | Kết quả thực tế | Đánh giá |
| :--- | :--- | :--- | :---: |
| **Topological Sort 5 Phases** | 5 Phase đăng ký lộn xộn kèm 4 edges | Sắp xếp chính xác: `[prepare, simulate, finalize, graph, submit]` | **ĐẠT** |
| **2-Node Direct Cycle** | `PhaseA <-> PhaseB` | `Err(PhaseCycleDetected("phase.a, phase.b"))` | **ĐẠT** |
| **Missing Dependency** | Nối cạnh tới Phase chưa đăng ký | `Err(PhaseNotFound("phase.b"))` | **ĐẠT** |

---

## 4. Phân Tích Hiệu Suất & Tính Tất Định
- **Thời gian thực thi:** `~93 µs`
- **Độ phức tạp:** $O(V + E)$ với $V$ là số Phase và $E$ là số cạnh phụ thuộc.
- **Tính tất định (Deterministic Tie-breaking):** Khi có nhiều Phase cùng có `in_degree == 0`, hệ thống sử dụng sắp xếp chữ cái (*Lexicographical sort*) trước khi đẩy vào hàng đợi Kahn, đảm bảo thứ tự luôn đồng nhất trên mọi nền tảng.
- **Trạng thái:** **ĐẠT (PASS ✅)**
