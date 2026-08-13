# Deferred destruction

`DeferredDestructionQueue<T>` giữ một backing object đến sau
`SubmissionId` cuối cùng sử dụng nó. Host gọi `drain_completed` sau khi cập
nhật `SubmissionTracker`, nhận lại các object an toàn để drop/remove.

Queue không tự biết `wgpu::Buffer`, `Texture` hay registry handle; generic
boundary này giúp core không sở hữu vòng đời backend thay cho host.
