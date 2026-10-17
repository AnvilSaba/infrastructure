#!/usr/bin/env -S mise exec -- cargo -Zscript
---cargo
[package]
edition = "2024"
---

use std::{error::Error, fs, path::Path, process::Command};

fn main() -> Result<(), Box<dyn Error>> {
    render(Path::new("."))
}

fn render(directory: &Path) -> Result<(), Box<dyn Error>> {
    if ["kustomization.yaml", "kustomization.yml", "Kustomization"]
        .iter()
        .any(|name| directory.join(name).is_file())
    {
        let output = Command::new("kustomize").arg("build").arg(directory).output()?;
        if !output.status.success() {
            return Err(format!("{}: {}", directory.display(), String::from_utf8_lossy(&output.stderr)).into());
        }
        println!("描画成功: {}", directory.display());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if entry.file_type()?.is_dir() && !name.starts_with('.') && !matches!(name.as_ref(), "node_modules" | "target")
        {
            render(&entry.path())?;
        }
    }
    Ok(())
}
