# IFOL GPU: resource lifecycle API hiện tại

Registry có remove API cho texture, render pipeline, compute pipeline, mesh, bind
group và buffer. Remove tăng resource version để bundle/compiled artifact cũ không
được coi là còn hợp lệ.

Mọi texture insertion đều yêu cầu descriptor hoặc owned-resource contract; không
còn đường insert texture cũ làm mất descriptor metadata.
