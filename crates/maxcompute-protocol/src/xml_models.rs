//! XML-serializable models for the MaxCompute (ODPS) REST API.
//!
//! All models use `quick-xml` with serde for serialization/deserialization.
//! Response types implement `to_xml()`, request types implement `from_xml()`.

use crate::error::{McError, McResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};

// ===========================================================================
// SubmitInstanceRequest
// ===========================================================================

/// Request to submit a new SQL instance (job) to MaxCompute.
///
/// ```xml
/// <Instance>
///   <Job>
///     <Priority>9</Priority>
///     <RunMode>Sequence</RunMode>
///     <Tasks>
///       <SQL Name="AnonymousSQLTask">
///         <Name>AnonymousSQLTask</Name>
///         <Query><![CDATA[SELECT * FROM table;]]></Query>
///         <Config>
///           <Property><Name>settings</Name><Value>{"key":"val"}</Value></Property>
///         </Config>
///       </SQL>
///     </Tasks>
///   </Job>
/// </Instance>
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Instance")]
pub struct SubmitInstanceRequest {
    #[serde(rename = "Job")]
    pub job: Job,
}

impl SubmitInstanceRequest {
    /// Create a new submit instance request with a single SQL task.
    pub fn new(sql: &str, priority: u32, settings: Option<serde_json::Value>) -> Self {
        let props = settings.map(|s| SqlConfigXml {
            properties: vec![PropertyXml {
                name: "settings".to_string(),
                value: s.to_string(),
            }],
        });

        Self {
            job: Job {
                priority,
                run_mode: "Sequence".to_string(),
                tasks: Tasks {
                    sql_tasks: vec![SqlTask {
                        name: "AnonymousSQLTask".to_string(),
                        inner_name: "AnonymousSQLTask".to_string(),
                        query: sql.to_string(),
                        config: props,
                    }],
                },
            },
        }
    }

    /// Parse from XML string.
    pub fn from_xml(xml: &str) -> McResult<Self> {
        quick_xml::de::from_str(xml).map_err(|e| McError::XmlError(e.to_string()))
    }

    /// Get the SQL query from the first SQL task.
    pub fn sql(&self) -> &str {
        &self.job.tasks.sql_tasks[0].query
    }

    /// Get the priority.
    pub fn priority(&self) -> u32 {
        self.job.priority
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    #[serde(rename = "Priority")]
    pub priority: u32,
    #[serde(rename = "RunMode")]
    pub run_mode: String,
    #[serde(rename = "Tasks")]
    pub tasks: Tasks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tasks {
    #[serde(rename = "SQL")]
    pub sql_tasks: Vec<SqlTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlTask {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "Name")]
    pub inner_name: String,
    #[serde(rename = "Query")]
    pub query: String,
    #[serde(rename = "Config", skip_serializing_if = "Option::is_none")]
    pub config: Option<SqlConfigXml>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlConfigXml {
    #[serde(rename = "Property")]
    pub properties: Vec<PropertyXml>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyXml {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Value")]
    pub value: String,
}

// ===========================================================================
// InstanceResponse
// ===========================================================================

/// Response containing instance status information.
///
/// ```xml
/// <Instance>
///   <Name>instance_id</Name>
///   <Owner>ALIYUN$roris</Owner>
///   <StartTime>2024-01-01T00:00:00Z</StartTime>
///   <EndTime>2024-01-01T00:00:01Z</EndTime>
///   <Status>Terminated</Status>
/// </Instance>
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Instance")]
pub struct InstanceResponse {
    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "Owner")]
    pub owner: String,

    #[serde(rename = "StartTime")]
    pub start_time: String,

    #[serde(rename = "EndTime", skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,

    #[serde(rename = "Status")]
    pub status: String,
}

impl InstanceResponse {
    /// Create a new instance response.
    pub fn new(
        name: impl Into<String>,
        owner: impl Into<String>,
        start_time: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            owner: owner.into(),
            start_time: start_time.into(),
            end_time: None,
            status: status.into(),
        }
    }

    /// Serialize to XML string.
    pub fn to_xml(&self) -> McResult<String> {
        quick_xml::se::to_string(&self).map_err(|e| McError::XmlError(e.to_string()))
    }

    /// Parse from XML string.
    pub fn from_xml(xml: &str) -> McResult<Self> {
        quick_xml::de::from_str(xml).map_err(|e| McError::XmlError(e.to_string()))
    }
}

// ===========================================================================
// TaskStatusResponse
// ===========================================================================

/// Response containing task status information for an instance.
///
/// ```xml
/// <Instance>
///   <Tasks>
///     <Task Name="AnonymousSQLTask" Type="SQL" Status="Success"/>
///   </Tasks>
/// </Instance>
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Instance")]
pub struct TaskStatusResponse {
    #[serde(rename = "Tasks")]
    pub tasks: TaskList,
}

impl TaskStatusResponse {
    /// Create a new task status response.
    pub fn new(tasks: Vec<TaskInfo>) -> Self {
        Self {
            tasks: TaskList { tasks },
        }
    }

    /// Serialize to XML string.
    pub fn to_xml(&self) -> McResult<String> {
        quick_xml::se::to_string(&self).map_err(|e| McError::XmlError(e.to_string()))
    }

    /// Parse from XML string.
    pub fn from_xml(xml: &str) -> McResult<Self> {
        quick_xml::de::from_str(xml).map_err(|e| McError::XmlError(e.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskList {
    #[serde(rename = "Task")]
    pub tasks: Vec<TaskInfo>,
}

/// A single task status with XML attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Type")]
    pub task_type: String,
    #[serde(rename = "@Status")]
    pub status: String,
}

// ===========================================================================
// TaskResultResponse
// ===========================================================================

/// Response containing task result data for an instance.
///
/// ```xml
/// <Instance>
///   <Tasks>
///     <Task Type="SQL">
///       <Name>AnonymousSQLTask</Name>
///       <Status>Success</Status>
///       <Result><![CDATA[csv_data_here]]></Result>
///       <Result>
///         <SelectResultStatus>OK</SelectResultStatus>
///         <IsSelect>true</IsSelect>
///       </Result>
///     </Task>
///   </Tasks>
/// </Instance>
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Instance")]
pub struct TaskResultResponse {
    #[serde(rename = "Tasks")]
    pub tasks: TaskResultList,
}

impl TaskResultResponse {
    /// Create a new task result response.
    pub fn new(tasks: Vec<TaskResult>) -> Self {
        Self {
            tasks: TaskResultList { tasks },
        }
    }

    /// Serialize to XML string.
    pub fn to_xml(&self) -> McResult<String> {
        quick_xml::se::to_string(&self).map_err(|e| McError::XmlError(e.to_string()))
    }

    /// Parse from XML string.
    pub fn from_xml(xml: &str) -> McResult<Self> {
        quick_xml::de::from_str(xml).map_err(|e| McError::XmlError(e.to_string()))
    }

    /// Get the result data (CSV text) from the first task, if available.
    pub fn result_data(&self) -> Option<&str> {
        self.tasks
            .tasks
            .first()
            .and_then(|task| task.results.iter().find_map(|r| r.text.as_deref()))
    }

    /// Get the select result status from the first task, if available.
    pub fn select_result_status(&self) -> Option<&str> {
        self.tasks.tasks.first().and_then(|task| {
            task.results
                .iter()
                .find_map(|r| r.select_result_status.as_deref())
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResultList {
    #[serde(rename = "Task")]
    pub tasks: Vec<TaskResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    #[serde(rename = "@Type")]
    pub task_type: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "Result")]
    pub results: Vec<ResultElement>,
}

/// Result content can be either plain text (CDATA) or structured metadata.
///
/// Uses `$value` to capture text content for CDATA results and named
/// fields for structured result metadata. When serializing, only the
/// populated variant is emitted.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ResultElement {
    /// Text content of the Result element (CDATA CSV data).
    #[serde(rename = "$value", default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Select result status (e.g. "OK").
    #[serde(
        rename = "SelectResultStatus",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub select_result_status: Option<String>,
    /// Whether the result is from a SELECT query.
    #[serde(rename = "IsSelect", default, skip_serializing_if = "Option::is_none")]
    pub is_select: Option<String>,
}

/// Structured result metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultDetails {
    #[serde(rename = "SelectResultStatus")]
    pub select_result_status: String,
    #[serde(rename = "IsSelect")]
    pub is_select: String,
}

// ===========================================================================
// TablesResponse
// ===========================================================================

/// Response listing all tables in a project.
///
/// ```xml
/// <Tables>
///   <Table>
///     <Name>table_name</Name>
///     <Owner>ALIYUN$roris</Owner>
///     <CreationTime>2024-01-01T00:00:00Z</CreationTime>
///     <LastModifiedTime>2024-01-01T00:00:00Z</LastModifiedTime>
///     <Type>MANUAL</Type>
///   </Table>
/// </Tables>
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Tables")]
pub struct TablesResponse {
    /// In quick-xml, if the root element is `<Tables>`, the struct itself
    /// is serialized as `<Tables>` and child elements are mapped inside.
    /// But `<Table>` elements are repeated siblings. quick-xml serde
    /// handles this by using `$value` or a Vec named after the element.
    #[serde(rename = "Table")]
    pub tables: Vec<TableSummary>,
}

impl TablesResponse {
    /// Create a new tables response.
    pub fn new(tables: Vec<TableSummary>) -> Self {
        Self { tables }
    }

    /// Serialize to XML string.
    pub fn to_xml(&self) -> McResult<String> {
        quick_xml::se::to_string(&self).map_err(|e| McError::XmlError(e.to_string()))
    }

    /// Parse from XML string.
    pub fn from_xml(xml: &str) -> McResult<Self> {
        quick_xml::de::from_str(xml).map_err(|e| McError::XmlError(e.to_string()))
    }
}

/// Summary information for a single table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSummary {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Owner")]
    pub owner: String,
    #[serde(rename = "CreationTime")]
    pub creation_time: String,
    #[serde(rename = "LastModifiedTime")]
    pub last_modified_time: String,
    #[serde(rename = "Type")]
    pub table_type: String,
}

// ===========================================================================
// TableDetailResponse
// ===========================================================================

/// Detailed table information including schema.
///
/// ```xml
/// <Table>
///   <Name>table_name</Name>
///   <Owner>ALIYUN$roris</Owner>
///   <CreationTime>2024-01-01T00:00:00Z</CreationTime>
///   <LastModifiedTime>2024-01-01T00:00:00Z</LastModifiedTime>
///   <Type>MANUAL</Type>
///   <TableSchema>
///     <Columns>
///       <Column><Name>id</Name><Type>BIGINT</Type><Nullable>true</Nullable></Column>
///     </Columns>
///     <PartitionKeys>
///       <Column><Name>ds</Name><Type>STRING</Type><Nullable>true</Nullable></Column>
///     </PartitionKeys>
///   </TableSchema>
///   <RecordNum>0</RecordNum>
/// </Table>
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Table")]
pub struct TableDetailResponse {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Owner")]
    pub owner: String,
    #[serde(rename = "CreationTime")]
    pub creation_time: String,
    #[serde(rename = "LastModifiedTime")]
    pub last_modified_time: String,
    #[serde(rename = "Type")]
    pub table_type: String,
    #[serde(rename = "TableSchema", skip_serializing_if = "Option::is_none")]
    pub table_schema: Option<TableSchema>,
    #[serde(rename = "RecordNum")]
    pub record_num: u64,
}

impl TableDetailResponse {
    /// Create a new table detail response.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        owner: impl Into<String>,
        creation_time: impl Into<String>,
        last_modified_time: impl Into<String>,
        table_type: impl Into<String>,
        columns: Vec<ColumnDef>,
        partition_keys: Vec<ColumnDef>,
        record_num: u64,
    ) -> Self {
        let table_schema = if columns.is_empty() && partition_keys.is_empty() {
            None
        } else {
            Some(TableSchema {
                columns: ColumnList { columns },
                partition_keys: Some(PartitionKeys {
                    columns: partition_keys,
                }),
            })
        };

        Self {
            name: name.into(),
            owner: owner.into(),
            creation_time: creation_time.into(),
            last_modified_time: last_modified_time.into(),
            table_type: table_type.into(),
            table_schema,
            record_num,
        }
    }

    /// Serialize to XML string.
    pub fn to_xml(&self) -> McResult<String> {
        quick_xml::se::to_string(&self).map_err(|e| McError::XmlError(e.to_string()))
    }

    /// Parse from XML string.
    pub fn from_xml(xml: &str) -> McResult<Self> {
        quick_xml::de::from_str(xml).map_err(|e| McError::XmlError(e.to_string()))
    }
}

/// Schema definition for a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    #[serde(rename = "Columns")]
    pub columns: ColumnList,
    #[serde(rename = "PartitionKeys", skip_serializing_if = "Option::is_none")]
    pub partition_keys: Option<PartitionKeys>,
}

/// Container for column definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnList {
    #[serde(rename = "Column")]
    pub columns: Vec<ColumnDef>,
}

/// Container for partition key definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionKeys {
    #[serde(rename = "Column")]
    pub columns: Vec<ColumnDef>,
}

/// A single column or partition key definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub column_type: String,
    #[serde(rename = "Nullable")]
    pub nullable: String,
}

// ===========================================================================
// ProjectResponse
// ===========================================================================

/// Response containing project metadata.
///
/// ```xml
/// <Project>
///   <Name>project_name</Name>
///   <Owner>ALIYUN$roris</Owner>
///   <CreationTime>2024-01-01T00:00:00Z</CreationTime>
///   <LastModifiedTime>2024-01-01T00:00:00Z</LastModifiedTime>
///   <Status>AVAILABLE</Status>
///   <RegionId>cn-hangzhou</RegionId>
/// </Project>
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Project")]
pub struct ProjectResponse {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Owner")]
    pub owner: String,
    #[serde(rename = "CreationTime")]
    pub creation_time: String,
    #[serde(rename = "LastModifiedTime")]
    pub last_modified_time: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "RegionId")]
    pub region_id: String,
}

impl ProjectResponse {
    /// Create a new project response.
    pub fn new(
        name: impl Into<String>,
        owner: impl Into<String>,
        creation_time: impl Into<String>,
        last_modified_time: impl Into<String>,
        status: impl Into<String>,
        region_id: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            owner: owner.into(),
            creation_time: creation_time.into(),
            last_modified_time: last_modified_time.into(),
            status: status.into(),
            region_id: region_id.into(),
        }
    }

    /// Serialize to XML string.
    pub fn to_xml(&self) -> McResult<String> {
        quick_xml::se::to_string(&self).map_err(|e| McError::XmlError(e.to_string()))
    }

    /// Parse from XML string.
    pub fn from_xml(xml: &str) -> McResult<Self> {
        quick_xml::de::from_str(xml).map_err(|e| McError::XmlError(e.to_string()))
    }
}

// ===========================================================================
// McErrorResponse
// ===========================================================================

/// Error response returned by MaxCompute API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McErrorResponse {
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Message")]
    pub message: String,
    #[serde(rename = "RequestId")]
    pub request_id: String,
}

// ===========================================================================
// Legacy deserialization-only types (for backward compat via extract_sql_from_body)
// ===========================================================================

/// Simplified XML structure for extracting SQL from request bodies.
/// Uses `Option` on all fields to be lenient about partial XML.
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitInstanceXml {
    #[serde(rename = "Job")]
    pub job: Option<JobXml>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JobXml {
    #[serde(rename = "Priority")]
    pub priority: Option<i32>,
    #[serde(rename = "RunMode")]
    pub run_mode: Option<String>,
    #[serde(rename = "Tasks")]
    pub tasks: Option<TasksXml>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TasksXml {
    #[serde(rename = "SQL")]
    pub sql: Option<SqlTaskXml>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SqlTaskXml {
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "Query")]
    pub query: Option<String>,
    #[serde(rename = "Config")]
    pub config: Option<SqlConfigXml>,
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Return the current time in ISO 8601 / RFC 3339 format.
pub fn now_iso8601() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Serialize a value to an XML string with an XML declaration prefix.
pub fn to_xml<T: Serialize>(value: &T) -> Result<String, String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    match quick_xml::se::to_string(value) {
        Ok(s) => {
            xml.push_str(&s);
            Ok(xml)
        }
        Err(e) => Err(format!("XML serialization error: {}", e)),
    }
}

/// Extract SQL from a submitted HTTP body. Handles both XML and raw SQL.
pub fn extract_sql_from_body(body: &[u8]) -> String {
    let body_str = String::from_utf8_lossy(body);
    let trimmed = body_str.trim();

    // Try XML parsing first
    if trimmed.starts_with('<') {
        if let Ok(submit) = quick_xml::de::from_str::<SubmitInstanceXml>(trimmed) {
            if let Some(job) = submit.job {
                if let Some(tasks) = job.tasks {
                    if let Some(sql) = tasks.sql {
                        if let Some(query) = sql.query {
                            return query.trim().to_string();
                        }
                    }
                }
            }
        }
    }

    // Fallback: treat as raw SQL
    trimmed.to_string()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // SubmitInstanceRequest
    // -----------------------------------------------------------------------

    #[test]
    fn test_submit_instance_request_serialize() {
        let req = SubmitInstanceRequest::new("SELECT 1", 9, None);
        let xml = quick_xml::se::to_string(&req).expect("serialize");
        assert!(xml.contains("<Instance>"));
        assert!(xml.contains("<Priority>9</Priority>"));
        assert!(xml.contains("<RunMode>Sequence</RunMode>"));
        assert!(xml.contains("<Query>SELECT 1</Query>"));
        assert!(xml.contains("</Instance>"));
    }

    #[test]
    fn test_submit_instance_request_from_xml() {
        let xml = r#"<Instance>
  <Job>
    <Priority>9</Priority>
    <RunMode>Sequence</RunMode>
    <Tasks>
      <SQL Name="AnonymousSQLTask">
        <Name>AnonymousSQLTask</Name>
        <Query>SELECT * FROM table;</Query>
      </SQL>
    </Tasks>
  </Job>
</Instance>"#;
        let req = SubmitInstanceRequest::from_xml(xml).expect("deserialize");
        assert_eq!(req.sql(), "SELECT * FROM table;");
        assert_eq!(req.priority(), 9);
        assert_eq!(req.job.run_mode, "Sequence");
        assert_eq!(req.job.tasks.sql_tasks.len(), 1);
    }

    #[test]
    fn test_submit_instance_request_with_settings() {
        let settings = serde_json::json!({"odps.sql.submit.mode": "script"});
        let req = SubmitInstanceRequest::new("SELECT 1", 5, Some(settings));
        let xml = quick_xml::se::to_string(&req).expect("serialize");
        assert!(xml.contains("<Config>"));
        assert!(xml.contains("odps.sql.submit.mode"));
    }

    #[test]
    fn test_submit_instance_request_from_xml_with_config() {
        let xml = r#"<Instance>
  <Job>
    <Priority>5</Priority>
    <RunMode>Sequence</RunMode>
    <Tasks>
      <SQL Name="MyTask">
        <Name>MyTask</Name>
        <Query>SELECT COUNT(*) FROM t</Query>
        <Config>
          <Property><Name>settings</Name><Value>{"key":"val"}</Value></Property>
        </Config>
      </SQL>
    </Tasks>
  </Job>
</Instance>"#;
        let req = SubmitInstanceRequest::from_xml(xml).expect("deserialize");
        assert_eq!(req.sql(), "SELECT COUNT(*) FROM t");
        let cfg = req.job.tasks.sql_tasks[0].config.as_ref().unwrap();
        assert_eq!(cfg.properties.len(), 1);
        assert_eq!(cfg.properties[0].name, "settings");
    }

    #[test]
    fn test_submit_instance_request_round_trip() {
        let req = SubmitInstanceRequest::new(
            "SELECT id, name FROM users WHERE active = 1",
            8,
            Some(serde_json::json!({"timeout": "600"})),
        );
        let xml = quick_xml::se::to_string(&req).expect("serialize");
        let parsed = SubmitInstanceRequest::from_xml(&xml).expect("deserialize round-trip");
        assert_eq!(req.sql(), parsed.sql());
        assert_eq!(req.priority(), parsed.priority());
    }

    // -----------------------------------------------------------------------
    // InstanceResponse
    // -----------------------------------------------------------------------

    #[test]
    fn test_instance_response_to_xml() {
        let resp = InstanceResponse {
            name: "20240101000001abcdef".to_string(),
            owner: "ALIYUN$roris".to_string(),
            start_time: "2024-01-01T00:00:00Z".to_string(),
            end_time: Some("2024-01-01T00:00:01Z".to_string()),
            status: "Terminated".to_string(),
        };
        let xml = resp.to_xml().expect("serialize");
        assert!(xml.contains("<Instance>"));
        assert!(xml.contains("<Name>20240101000001abcdef</Name>"));
        assert!(xml.contains("<Owner>ALIYUN$roris</Owner>"));
        assert!(xml.contains("<Status>Terminated</Status>"));
    }

    #[test]
    fn test_instance_response_from_xml() {
        let xml = r#"<Instance>
  <Name>inst_001</Name>
  <Owner>ALIYUN$test</Owner>
  <StartTime>2024-06-01T12:00:00Z</StartTime>
  <Status>Running</Status>
</Instance>"#;
        let resp = InstanceResponse::from_xml(xml).expect("deserialize");
        assert_eq!(resp.name, "inst_001");
        assert_eq!(resp.owner, "ALIYUN$test");
        assert_eq!(resp.status, "Running");
        assert!(resp.end_time.is_none());
    }

    #[test]
    fn test_instance_response_round_trip() {
        let resp =
            InstanceResponse::new("i-abc", "ALIYUN$roris", "2024-01-01T00:00:00Z", "Running");
        let xml = resp.to_xml().expect("serialize");
        let parsed = InstanceResponse::from_xml(&xml).expect("deserialize round-trip");
        assert_eq!(parsed.name, "i-abc");
        assert_eq!(parsed.status, "Running");
    }

    // -----------------------------------------------------------------------
    // TaskStatusResponse
    // -----------------------------------------------------------------------

    #[test]
    fn test_task_status_response_to_xml() {
        let task = TaskInfo {
            name: "AnonymousSQLTask".to_string(),
            task_type: "SQL".to_string(),
            status: "Success".to_string(),
        };
        let resp = TaskStatusResponse::new(vec![task]);
        let xml = resp.to_xml().expect("serialize");
        assert!(xml.contains("<Instance>"));
        assert!(xml.contains("<Tasks>"));
        assert!(xml.contains(r#"Name="AnonymousSQLTask""#));
        assert!(xml.contains(r#"Type="SQL""#));
        assert!(xml.contains(r#"Status="Success""#));
    }

    #[test]
    fn test_task_status_response_from_xml() {
        let xml = r#"<Instance>
  <Tasks>
    <Task Name="AnonymousSQLTask" Type="SQL" Status="Success"/>
  </Tasks>
</Instance>"#;
        let resp = TaskStatusResponse::from_xml(xml).expect("deserialize");
        assert_eq!(resp.tasks.tasks.len(), 1);
        assert_eq!(resp.tasks.tasks[0].name, "AnonymousSQLTask");
        assert_eq!(resp.tasks.tasks[0].task_type, "SQL");
        assert_eq!(resp.tasks.tasks[0].status, "Success");
    }

    #[test]
    fn test_task_status_response_round_trip() {
        let tasks = vec![
            TaskInfo {
                name: "Task1".to_string(),
                task_type: "SQL".to_string(),
                status: "Running".to_string(),
            },
            TaskInfo {
                name: "Task2".to_string(),
                task_type: "SQL".to_string(),
                status: "Waiting".to_string(),
            },
        ];
        let resp = TaskStatusResponse::new(tasks);
        let xml = resp.to_xml().expect("serialize");
        let parsed = TaskStatusResponse::from_xml(&xml).expect("deserialize round-trip");
        assert_eq!(parsed.tasks.tasks.len(), 2);
        assert_eq!(parsed.tasks.tasks[0].name, "Task1");
    }

    // -----------------------------------------------------------------------
    // TaskResultResponse
    // -----------------------------------------------------------------------

    #[test]
    fn test_task_result_response_from_xml() {
        let xml = r#"<Instance>
  <Tasks>
    <Task Type="SQL">
      <Name>AnonymousSQLTask</Name>
      <Status>Success</Status>
      <Result>id,name&#10;1,Alice&#10;2,Bob</Result>
      <Result>
        <SelectResultStatus>OK</SelectResultStatus>
        <IsSelect>true</IsSelect>
      </Result>
    </Task>
  </Tasks>
</Instance>"#;
        let resp = TaskResultResponse::from_xml(xml).expect("deserialize");
        assert_eq!(resp.tasks.tasks.len(), 1);
        assert_eq!(resp.tasks.tasks[0].name, "AnonymousSQLTask");
        assert_eq!(resp.tasks.tasks[0].results.len(), 2);

        // First result should be text
        assert!(resp.tasks.tasks[0].results[0].text.is_some());
        assert!(
            resp.tasks.tasks[0].results[0]
                .text
                .as_ref()
                .unwrap()
                .contains("Alice")
        );

        // Second result should be structured
        assert!(
            resp.tasks.tasks[0].results[1]
                .select_result_status
                .is_some()
        );
        assert_eq!(
            resp.tasks.tasks[0].results[1]
                .select_result_status
                .as_deref(),
            Some("OK")
        );
        assert_eq!(
            resp.tasks.tasks[0].results[1].is_select.as_deref(),
            Some("true")
        );
    }

    #[test]
    fn test_task_result_response_convenience_methods() {
        let resp = TaskResultResponse {
            tasks: TaskResultList {
                tasks: vec![TaskResult {
                    task_type: "SQL".to_string(),
                    name: "AnonymousSQLTask".to_string(),
                    status: "Success".to_string(),
                    results: vec![
                        ResultElement {
                            text: Some("col1,col2\n1,2".to_string()),
                            select_result_status: None,
                            is_select: None,
                        },
                        ResultElement {
                            text: None,
                            select_result_status: Some("OK".to_string()),
                            is_select: Some("true".to_string()),
                        },
                    ],
                }],
            },
        };
        assert_eq!(resp.result_data(), Some("col1,col2\n1,2"));
        assert_eq!(resp.select_result_status(), Some("OK"));
    }

    #[test]
    fn test_task_result_response_to_xml() {
        let resp = TaskResultResponse {
            tasks: TaskResultList {
                tasks: vec![TaskResult {
                    task_type: "SQL".to_string(),
                    name: "MyTask".to_string(),
                    status: "Success".to_string(),
                    results: vec![
                        ResultElement {
                            text: Some("data".to_string()),
                            select_result_status: None,
                            is_select: None,
                        },
                        ResultElement {
                            text: None,
                            select_result_status: Some("OK".to_string()),
                            is_select: Some("false".to_string()),
                        },
                    ],
                }],
            },
        };
        let xml = resp.to_xml().expect("serialize");
        assert!(xml.contains("<Instance>"));
        assert!(xml.contains("<Name>MyTask</Name>"));
        assert!(xml.contains("<Result>data</Result>"));
        assert!(xml.contains("<SelectResultStatus>OK</SelectResultStatus>"));
    }

    #[test]
    fn test_task_result_response_round_trip() {
        let original = TaskResultResponse {
            tasks: TaskResultList {
                tasks: vec![TaskResult {
                    task_type: "SQL".to_string(),
                    name: "T1".to_string(),
                    status: "Success".to_string(),
                    results: vec![
                        ResultElement {
                            text: Some("csv,data".to_string()),
                            select_result_status: None,
                            is_select: None,
                        },
                        ResultElement {
                            text: None,
                            select_result_status: Some("OK".to_string()),
                            is_select: Some("true".to_string()),
                        },
                    ],
                }],
            },
        };
        let xml = original.to_xml().expect("serialize");
        let parsed = TaskResultResponse::from_xml(&xml).expect("deserialize round-trip");
        assert_eq!(parsed.result_data(), original.result_data());
        assert_eq!(
            parsed.select_result_status(),
            original.select_result_status()
        );
    }

    // -----------------------------------------------------------------------
    // TablesResponse
    // -----------------------------------------------------------------------

    #[test]
    fn test_tables_response_to_xml() {
        let tables = vec![
            TableSummary {
                name: "table1".to_string(),
                owner: "ALIYUN$roris".to_string(),
                creation_time: "2024-01-01T00:00:00Z".to_string(),
                last_modified_time: "2024-01-01T00:00:00Z".to_string(),
                table_type: "MANUAL".to_string(),
            },
            TableSummary {
                name: "table2".to_string(),
                owner: "ALIYUN$roris".to_string(),
                creation_time: "2024-01-02T00:00:00Z".to_string(),
                last_modified_time: "2024-01-02T00:00:00Z".to_string(),
                table_type: "MANUAL".to_string(),
            },
        ];
        let resp = TablesResponse::new(tables);
        let xml = resp.to_xml().expect("serialize");
        assert!(xml.contains("<Table>"));
        assert!(xml.contains("<Name>table1</Name>"));
        assert!(xml.contains("<Name>table2</Name>"));
    }

    #[test]
    fn test_tables_response_from_xml() {
        let xml = r#"<Tables>
  <Table>
    <Name>test_table</Name>
    <Owner>ALIYUN$roris</Owner>
    <CreationTime>2024-01-01T00:00:00Z</CreationTime>
    <LastModifiedTime>2024-01-01T00:00:00Z</LastModifiedTime>
    <Type>MANUAL</Type>
  </Table>
</Tables>"#;
        let resp = TablesResponse::from_xml(xml).expect("deserialize");
        assert_eq!(resp.tables.len(), 1);
        assert_eq!(resp.tables[0].name, "test_table");
        assert_eq!(resp.tables[0].table_type, "MANUAL");
    }

    #[test]
    fn test_tables_response_empty() {
        let resp = TablesResponse::new(vec![]);
        let xml = resp.to_xml().expect("serialize");
        // quick-xml may produce either <Tables/> or <Tables></Tables>
        assert!(
            xml.contains("Tables"),
            "Expected Tables element in: {}",
            xml
        );
    }

    // -----------------------------------------------------------------------
    // TableDetailResponse
    // -----------------------------------------------------------------------

    #[test]
    fn test_table_detail_response_to_xml() {
        let resp = TableDetailResponse::new(
            "test_table",
            "ALIYUN$roris",
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00Z",
            "MANUAL",
            vec![ColumnDef {
                name: "id".to_string(),
                column_type: "BIGINT".to_string(),
                nullable: "true".to_string(),
            }],
            vec![ColumnDef {
                name: "ds".to_string(),
                column_type: "STRING".to_string(),
                nullable: "true".to_string(),
            }],
            42,
        );
        let xml = resp.to_xml().expect("serialize");
        assert!(xml.contains("<Table>"));
        assert!(xml.contains("<Name>test_table</Name>"));
        assert!(xml.contains("<RecordNum>42</RecordNum>"));
        assert!(xml.contains("<TableSchema>"));
        assert!(xml.contains("<Columns>"));
        assert!(xml.contains("<PartitionKeys>"));
    }

    #[test]
    fn test_table_detail_response_from_xml() {
        let xml = r#"<Table>
  <Name>users</Name>
  <Owner>ALIYUN$admin</Owner>
  <CreationTime>2024-01-01T00:00:00Z</CreationTime>
  <LastModifiedTime>2024-01-01T00:00:00Z</LastModifiedTime>
  <Type>MANUAL</Type>
  <TableSchema>
    <Columns>
      <Column><Name>id</Name><Type>BIGINT</Type><Nullable>true</Nullable></Column>
      <Column><Name>name</Name><Type>STRING</Type><Nullable>false</Nullable></Column>
    </Columns>
    <PartitionKeys>
      <Column><Name>ds</Name><Type>STRING</Type><Nullable>true</Nullable></Column>
    </PartitionKeys>
  </TableSchema>
  <RecordNum>1000</RecordNum>
</Table>"#;
        let resp = TableDetailResponse::from_xml(xml).expect("deserialize");
        assert_eq!(resp.name, "users");
        assert_eq!(resp.record_num, 1000);
        let schema = resp.table_schema.expect("should have schema");
        assert_eq!(schema.columns.columns.len(), 2);
        assert_eq!(schema.columns.columns[0].name, "id");
        assert_eq!(schema.columns.columns[0].column_type, "BIGINT");
        assert_eq!(schema.columns.columns[1].name, "name");
        assert_eq!(schema.columns.columns[1].nullable, "false");
        let partitions = schema.partition_keys.expect("should have partition keys");
        assert_eq!(partitions.columns.len(), 1);
        assert_eq!(partitions.columns[0].name, "ds");
    }

    #[test]
    fn test_table_detail_response_no_schema() {
        let resp = TableDetailResponse::new(
            "empty",
            "ALIYUN$roris",
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00Z",
            "MANUAL",
            vec![],
            vec![],
            0,
        );
        let xml = resp.to_xml().expect("serialize");
        assert!(!xml.contains("<TableSchema>"), "should omit empty schema");
    }

    // -----------------------------------------------------------------------
    // ProjectResponse
    // -----------------------------------------------------------------------

    #[test]
    fn test_project_response_to_xml() {
        let resp = ProjectResponse::new(
            "my_project",
            "ALIYUN$roris",
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00Z",
            "AVAILABLE",
            "cn-hangzhou",
        );
        let xml = resp.to_xml().expect("serialize");
        assert!(xml.contains("<Project>"));
        assert!(xml.contains("<Name>my_project</Name>"));
        assert!(xml.contains("<Status>AVAILABLE</Status>"));
        assert!(xml.contains("<RegionId>cn-hangzhou</RegionId>"));
    }

    #[test]
    fn test_project_response_from_xml() {
        let xml = r#"<Project>
  <Name>analytics</Name>
  <Owner>ALIYUN$team</Owner>
  <CreationTime>2024-03-15T10:30:00Z</CreationTime>
  <LastModifiedTime>2024-03-15T11:00:00Z</LastModifiedTime>
  <Status>AVAILABLE</Status>
  <RegionId>cn-beijing</RegionId>
</Project>"#;
        let resp = ProjectResponse::from_xml(xml).expect("deserialize");
        assert_eq!(resp.name, "analytics");
        assert_eq!(resp.status, "AVAILABLE");
        assert_eq!(resp.region_id, "cn-beijing");
    }

    #[test]
    fn test_project_response_round_trip() {
        let resp = ProjectResponse::new(
            "rt_test",
            "ALIYUN$test",
            "2024-06-01T00:00:00Z",
            "2024-06-01T12:00:00Z",
            "AVAILABLE",
            "cn-hangzhou",
        );
        let xml = resp.to_xml().expect("serialize");
        let parsed = ProjectResponse::from_xml(&xml).expect("deserialize round-trip");
        assert_eq!(parsed.name, "rt_test");
        assert_eq!(parsed.region_id, "cn-hangzhou");
    }

    // -----------------------------------------------------------------------
    // extract_sql_from_body
    // -----------------------------------------------------------------------

    #[test]
    fn test_extract_sql_from_xml() {
        let xml = r#"<Instance><Job><Priority>9</Priority><Tasks><SQL><Name>AnonymousSQLTask</Name><Query>SELECT * FROM t</Query></SQL></Tasks></Job></Instance>"#;
        let sql = extract_sql_from_body(xml.as_bytes());
        assert_eq!(sql, "SELECT * FROM t");
    }

    #[test]
    fn test_extract_sql_raw() {
        let sql = extract_sql_from_body(b"SELECT 1");
        assert_eq!(sql, "SELECT 1");
    }

    // -----------------------------------------------------------------------
    // now_iso8601
    // -----------------------------------------------------------------------

    #[test]
    fn test_now_iso8601() {
        let ts = now_iso8601();
        assert!(ts.contains('T'));
        assert!(ts.ends_with('Z'));
    }

    // -----------------------------------------------------------------------
    // Error handling
    // -----------------------------------------------------------------------

    #[test]
    fn test_invalid_xml_returns_error() {
        let result = SubmitInstanceRequest::from_xml("not valid xml");
        assert!(result.is_err());
        match result {
            Err(McError::XmlError(_)) => {} // expected
            _ => panic!("Expected XmlError"),
        }
    }

    #[test]
    fn test_missing_field_xml_error() {
        let xml = r#"<Instance><Name>test</Name></Instance>"#;
        let result = InstanceResponse::from_xml(xml);
        assert!(result.is_err());
    }
}
