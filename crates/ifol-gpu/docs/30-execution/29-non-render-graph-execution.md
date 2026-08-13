# IFOL GPU: graph không có render target

Graph chỉ có copy hoặc compute node vẫn được encode và submit khi không có
render target/surface. Render target chỉ bắt buộc khi graph chứa render work.

Runtime test đã kiểm tra copy buffer-to-buffer thực tế trong graph `Screen` không có
surface, sau đó map buffer đích để xác nhận dữ liệu được sao chép.
