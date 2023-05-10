use std::collections::HashMap;

use serde::Deserialize;
use serde_json;

/// Root Node for Slice Deserialization
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FullSlice {
    pub object_slices: HashMap<String, Vec<JsonObjSlice>>,
    pub user_defined_types: Vec<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JsonObjSlice {
    pub target_obj: TargetObj,
    pub defined_by: serde_json::Value,
    pub invoked_calls: Vec<Call>,
    pub arg_to_calls: Vec<(Call, i32)>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TargetObj {
    pub name: String,
    pub type_full_name: String,
    pub literal: bool,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Call {
    pub receiver: Option<String>,
    pub call_name: String,
    pub param_types: Vec<serde_json::Value>,
    pub return_type: String,
}

/// Data Structure used for internal representation of Slice
#[derive(Debug)]
pub struct ObjSlice {
    pub name: String,
    pub scope: String,
    pub type_name: String,
    pub invoked_calls: Vec<Call>,
    pub arg_to_calls: Vec<(Call, i32)>,
}
