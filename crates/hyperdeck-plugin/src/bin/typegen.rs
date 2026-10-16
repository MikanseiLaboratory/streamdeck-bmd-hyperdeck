fn main() -> anyhow::Result<()> {
    hyperdeck_plugin::force_link();
    let dest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../pi/src/generated/contracts.ts");
    streamdeck_plugin::export_typescript(&dest)?;
    let text = std::fs::read_to_string(&dest)?.replace("\ntype ", "\nexport type ");
    std::fs::write(&dest, text)?;
    Ok(())
}
