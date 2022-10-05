use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use eris::compound_proxy::{
    CallbackMsg, CompoundSimulationResponse, ExecuteMsg, InstantiateMsg, LpConfig, MigrateMsg,
    QueryMsg, RouteResponseItem,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(CallbackMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(CompoundSimulationResponse), &out_dir);
    export_schema(&schema_for!(RouteResponseItem), &out_dir);
    export_schema(&schema_for!(LpConfig), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);
}
