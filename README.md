# OpenPhilosophy

Local-first, append-only exploration history for humans and agents.

OpenPhilosophy 用稳定 ID、不可覆盖的记录和显式引用，保存探索中的失败、发现与依据。Rust Core 不依赖 LLM，也不需要外部服务。

**当前阶段：v0.1 开发预览。** 已实现 Rust Library、SQLite 存储和 JSON CLI。TypeScript SDK、MCP、REST API 和 Web UI 尚未实现。

## 安装

需要 Rust stable 工具链和 C 编译器（用于构建随依赖打包的 SQLite）。

```bash
git clone https://github.com/fuyangg/openphilosophy.git
cd openphilosophy
cargo install --path crates/ophi-cli --locked
ophi --help
```

## 第一次使用

创建一个探索档案，保存返回结果中的 `data.id`：

```bash
ophi --db ./ophi.db case create --title "排查偶发超时"
```

把下面的 `<case-id>` 替换为刚返回的 ID，再追加一条记录：

```bash
ophi --db ./ophi.db entry append <<'JSON'
{
  "case_id": "<case-id>",
  "kind": "failure",
  "content": "增加请求超时时间没有解决故障，连接池仍然耗尽。",
  "author": "human",
  "references": []
}
JSON
```

另一个终端或之后的 Session 使用同一数据库文件：

```bash
ophi --db ./ophi.db entry get <entry-id>
ophi --db ./ophi.db case get <case-id>
ophi --db ./ophi.db case history <case-id> --limit 100
```

修订通过新记录表达：新 Entry 的 `references` 填入
`{"target_id":"<old-entry-id>","relation":"supersedes"}`。旧 Entry 继续可以查询。

运行完整的双进程写入、引用、重新检索示例（需要 Python 3）：

```bash
python3 examples/debugging/demo.py
```

## 行为约定

- ID 和 UTC 时间戳由 Core 生成；外部请求不能指定它们。
- Case、Entry 和引用不提供修改或删除操作。
- 所有写入使用事务；引用目标必须已经存在并属于同一 Case。
- `kind`、`relation` 是开放字符串；`proof_submitted` 不会自动成为 `verified`。
- 历史按追加序号排序，分页游标仅用于原数据库、原 Case。
- stdin JSON 最多 1 MiB；Core 也限制序列化后单条记录最多 1 MiB。
- 未知 JSON 字段会被拒绝。字段、引用数量和分页另有边界限制。

成功输出 JSON 到 stdout：

```json
{"protocol_version":1,"data":{"entries":[],"next_cursor":null}}
```

失败输出 JSON 到 stderr，退出码为 1；参数错误退出码为 2。
`--help` 和 `--version` 输出普通文本。

完整约定见 [v0.1 规范](spec/v0.1.md) 和 [开放记录 JSON Schema](spec/record.schema.json)。

## 架构

```text
Agent / Human
    │ JSON stdin/stdout
    ▼
ophi CLI                 crates/ophi-cli
    │ Repository
    ▼
Rust Core + SQLite       crates/ophi-core
```

Repository 隔离历史读写语义与具体存储。当前仅实现 SQLite；尚未提供 JSON 导入导出命令。SQLite 文件格式不是公共交换协议。

数据库默认路径为当前目录下的 `ophi.db`。调用者应明确指定并保管自己的数据库路径；父目录需已存在。多进程写入由 SQLite 事务串行化，遇锁最多等待 5 秒后返回错误。当前适合本机文件系统上的小规模使用，尚未进行性能基准测试。

## 开发与验证

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo install cargo-audit --locked
cargo audit
```

测试覆盖重复 ID、不可覆盖、持久化、引用完整性、事务回滚、并发连接、分页、非法输入和独立 CLI 进程。

项目自身禁止 unsafe Rust；依赖可以内部使用 unsafe。数据库触发器用于防止意外改写，不是防篡改或身份权限系统：能修改数据库文件的人可以改变历史。`author` 是调用者声明，不是认证身份。请按普通本地数据文件管理访问权限和备份。

## 下一步

1. 用真实数学探索和故障排查任务验证历史模型。
2. 根据使用反馈稳定 v1 JSON 协议，增加明确的导出与迁移能力。
3. 加入通过 CLI JSON 协议调用的 TypeScript SDK。
4. 再接入 MCP 和多 Agent 场景。

## License

[MIT](LICENSE)
