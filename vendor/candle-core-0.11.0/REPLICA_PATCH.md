# Candle core 0.11.0: Metal F32 sum boundary

Upstream package: https://crates.io/crates/candle-core/0.11.0
Published archive SHA256:
`5ecb245093b0f791b89d3420c3df9c6d49c60ab63ba54db896bf8a3baf486706`.
Upstream VCS revision: `31f35b147389700ed2a178ee66a91c3cc25cc80d`,
path `candle-core`. The official versioned archive matched the existing registry
package file-for-file before modification. Its Apache-2.0 LICENSE is retained.
Registry sources are unmodified. The root Cargo patch resolves core to this one
directory; Candle nn and metal kernels remain locked at 0.11.0.

Local patch `metal-f32-sum-contiguous-v1` changes only
`src/metal_backend/mod.rs::MetalStorage::reduce_op`. For nonempty Metal F32 Sum
that would take the strided reducer, it copies the logical preserved/reduced
axis permutation into physical contiguous GPU storage and invokes the existing
suffix reducer. Offset and zero-stride layouts use the existing GPU copy path.
Public Tensor sum and Broadcast backward both reach this dispatch.
CPU/CUDA, other dtypes/ops, empty tensors and the existing contiguous fast path
are unchanged. No shader, host reduction, gradient detach or model change.

The original strided shader is **not fixed**. This is its replacement by an
equivalent GPU path, with an extra full-input temporary allocation and copy.
See `replica-metal-sum.patch` for the complete upstream-source delta.
