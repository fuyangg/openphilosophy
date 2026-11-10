# 数学探索示例

创建题为“证明候选命题”的 Case，依次追加：

1. kind=hypothesis，记录准确命题与前提。
2. kind=failed_attempt，记录失败推导，并用 investigates 引用命题。
3. kind=proof_submitted，记录证明文本，并用 supports 引用命题。
4. 经人工或独立验证后，追加 kind=verification，明确验证方法、结论及局限，用 verifies 引用提交的证明。

这些 kind/relation 都是约定字符串。Core 只保证历史与引用的完整性，不会因为名为 verification 就认定证明正确。

运行方式与 [故障排查示例](../debugging/demo.py) 相同：使用 case create 和 entry append，再按 Case 检索历史。
