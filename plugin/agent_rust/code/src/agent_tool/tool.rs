pub mod agent_tool;

pub trait AgentTool {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn param_schema(&self) -> serde_json::Value;
    fn execute(&self);
}
