# IFOL GPU: resource lifecycle API hiện tại

Registry có remove API cho texture, render pipeline, compute pipeline, mesh, bind
group và buffer. Remove tăng resource version để bundle/compiled artifact cũ không
được coi là còn hợp lệ.

Compatibility texture insert không có descriptor sẽ xóa descriptor metadata cũ;
host production nên dùng insert có descriptor để tránh mất thông tin compatibility.
