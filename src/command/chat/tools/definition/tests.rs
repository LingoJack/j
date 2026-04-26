use super::*;
use schemars::JsonSchema;
use serde::Deserialize;

/// 模拟 TodoWriteParams 结构（与 todo_write_tool.rs 中定义相同）
#[derive(Deserialize, JsonSchema)]
struct TodoItemParam {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    content: String,
    #[serde(default = "default_status")]
    status: String,
}

fn default_status() -> String {
    "pending".to_string()
}

#[derive(Deserialize, JsonSchema)]
struct TodoWriteParams {
    todos: Vec<TodoItemParam>,
    #[serde(default)]
    merge: bool,
}

/// 测试 schemars 生成的 schema 中 content 不再出现在 required 里
#[test]
fn test_schema_content_not_required() {
    let schema = schema_to_tool_params::<TodoWriteParams>();

    // 顶层 required 应包含 "todos"，不包含 "merge"（有 serde default）
    let required: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    assert!(
        required.contains(&"todos"),
        "todos should be required, got: {:?}",
        required
    );
    assert!(
        !required.contains(&"merge"),
        "merge should NOT be required (has serde default), got: {:?}",
        required
    );

    // todos items 里的 required：id 和 content 都有 serde(default)，不应出现
    let todos_schema = schema
        .get("properties")
        .and_then(|p| p.get("todos"))
        .and_then(|t| t.get("items"));

    assert!(todos_schema.is_some(), "todos.items should exist in schema");

    let item_required: Vec<&str> = todos_schema
        .and_then(|s| s.get("required"))
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    // 关键断言：content 不应在 required 里（因为 #[serde(default)]）
    assert!(
        !item_required.contains(&"content"),
        "content should NOT be required in todo items (has serde default), got required: {:?}",
        item_required
    );
    assert!(
        !item_required.contains(&"id"),
        "id should NOT be required in todo items (has serde default), got required: {:?}",
        item_required
    );
}

/// 测试 schema 内联后没有残留的 $ref
#[test]
fn test_schema_no_dangling_refs() {
    let schema = schema_to_tool_params::<TodoWriteParams>();
    let schema_str = serde_json::to_string(&schema).unwrap();
    assert!(
        !schema_str.contains("\"$ref\""),
        "Schema should not contain any $ref after inlining, got: {}",
        schema_str
    );
}

/// 测试 merge=true 时只传 id+status（不传 content）能正确反序列化
#[test]
fn test_merge_without_content_deserializes() {
    let json = r#"{"todos": [{"id": "1", "status": "completed"}], "merge": true}"#;
    let params: TodoWriteParams = serde_json::from_str(json).unwrap();
    assert!(params.merge);
    assert_eq!(params.todos.len(), 1);
    assert_eq!(params.todos[0].id, Some("1".to_string()));
    assert_eq!(params.todos[0].content, ""); // default empty string
    assert_eq!(params.todos[0].status, "completed");
}

/// 测试完整参数能正确反序列化
#[test]
fn test_full_params_deserialize() {
    let json = r#"{"todos": [{"id": "1", "content": "implement feature", "status": "in_progress"}, {"content": "write tests"}], "merge": false}"#;
    let params: TodoWriteParams = serde_json::from_str(json).unwrap();
    assert!(!params.merge);
    assert_eq!(params.todos.len(), 2);
    assert_eq!(params.todos[0].content, "implement feature");
    assert_eq!(params.todos[1].id, None);
    assert_eq!(params.todos[1].content, "write tests");
    assert_eq!(params.todos[1].status, "pending"); // default
}
