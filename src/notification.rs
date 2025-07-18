use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Params {
    Solution(SolutionParams),
    Project(ProjectParams),
}

#[derive(Serialize, Debug)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Params,
}

#[derive(Serialize, Debug)]
pub struct SolutionParams {
    pub solution: String,
}

#[derive(Serialize, Debug)]
pub struct ProjectParams {
    pub projects: Vec<String>,
}

impl Notification {
    pub fn serialize(self) -> String {
        let body = serde_json::to_string(&self).expect("Unable to serialize notification");
        add_content_length_header(&body)
    }
}

pub fn add_content_length_header(body: &str) -> String {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let full_message = format!("{}{}", header, body);

    full_message
}
