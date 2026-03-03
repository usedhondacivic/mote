//! Generates json schema files for the mote message types

use std::{env, fs, path::PathBuf};

use schemars::schema_for;

use mote_api::messages::{host_to_mote, mote_to_host};

fn main() -> anyhow::Result<()> {
    let host_to_mote_schema = serde_json::to_string_pretty(&schema_for!(host_to_mote::Message))?;
    let mote_to_host_schema = serde_json::to_string_pretty(&schema_for!(mote_to_host::Message))?;

    let out_dir = env::current_dir()?;

    fs::write(
        out_dir.join("host_to_mote_schema.json"),
        host_to_mote_schema,
    )?;
    fs::write(
        out_dir.join("mote_to_host_schema.json"),
        mote_to_host_schema,
    )?;

    Ok(())
}
