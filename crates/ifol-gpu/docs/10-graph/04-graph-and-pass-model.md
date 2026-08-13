# IFOL GPU: mô hình graph và pass

Graph mô tả GPU work, resource usage và dependency. Graph có thể chứa render,
compute, copy hoặc nested subgraph; compiler flatten nested graph thành plan
phẳng rồi encode theo thứ tự dependency.

## Quy tắc compile

1. kiểm tra node/resource/attachment và capability;
2. suy ra hazard edge từ usage overlap, đồng thời kiểm tra dependency tường minh;
3. topo-sort thành flat plan ổn định;
4. encode render/compute/copy theo segment phù hợp;
5. submit một command buffer hoặc boundary profiling do host chọn.

Cycle, resource thiếu, range/aspect sai, usage thiếu, attachment không tương
thích và layout metadata mismatch là lỗi typed trước submit.

Một graph không bị giới hạn vĩnh viễn thành một render pass. Render pass hiện
là policy encode cho target; graph vẫn là abstraction rộng hơn và có thể dùng
cho tính toán không có render target.

Resource usage có thể khai báo explicit hoặc được suy ra cho command built-in;
shader storage usage và semantic domain vẫn phải do host khai báo.
