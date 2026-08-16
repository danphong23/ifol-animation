# Private resource store

Các map GPU resource của `ResourceRegistry` đã được private hóa. Caller không
thể tự thay thế resource mà bỏ qua version tracking, descriptor metadata hoặc
ownership bookkeeping.

Public contract hiện gồm:

- API `insert_*_with_descriptor`/owned-resource và `remove_*` cho mutation;
- getter và `contains_*` cho lookup;
- `*_version`/`mark_*_changed` cho cache invalidation;
- descriptor/owned-resource API cho validation và lifetime.

Đây là mốc hoàn thành phần encapsulation của resource store. Các example,
test và benchmark đã dùng API chính thức; raw implementation map không nằm
trong public contract.
