use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema_with_title, remove_schemas, schema_for};

use eris_staking::yieldextractor::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, ShareResponse, StateResponse,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema_with_title(&schema_for!(InstantiateMsg), &out_dir, "InstantiateMsg");
    export_schema_with_title(&schema_for!(ExecuteMsg), &out_dir, "ExecuteMsg");
    export_schema_with_title(&schema_for!(QueryMsg), &out_dir, "QueryMsg");
    export_schema_with_title(&schema_for!(ConfigResponse), &out_dir, "ConfigResponse");

    export_schema_with_title(&schema_for!(StateResponse), &out_dir, "StateResponse");
    export_schema_with_title(&schema_for!(ShareResponse), &out_dir, "ShareResponse");

    export_schema_with_title(&schema_for!(ReceiveMsg), &out_dir, "ReceiveMsg");
}
