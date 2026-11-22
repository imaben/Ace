//! Bundle system compatible with Sequel Ace `.saBundle` packages.

use ace_core::{AceError, AceResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub name: String,
    pub category: Option<String>,
    pub trigger: Option<String>,
    pub input: BundleInput,
    pub output: BundleOutput,
    pub command: String,
    pub scope: Option<String>,
    pub key_equivalent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleInput {
    None,
    SelectedText,
    SelectedTableRows,
    CurrentWord,
    EntireContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleOutput {
    Discard,
    ReplaceSelection,
    InsertAsText,
    ShowAsHtml,
    ShowAsTooltip,
    ShowAsText,
}

#[derive(Debug, Clone)]
pub struct Bundle {
    pub id: Uuid,
    pub path: PathBuf,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone)]
pub struct BundleInputData {
    pub selected_text: Option<String>,
    pub table_tsv: Option<String>,
    pub current_word: Option<String>,
    pub entire_content: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BundleResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub output_kind: BundleOutput,
}

impl Bundle {
    pub fn load(dir: &Path) -> AceResult<Self> {
        let command_plist = dir.join("command.plist");
        if !command_plist.exists() {
            return Err(AceError::msg(format!(
                "missing command.plist in {}",
                dir.display()
            )));
        }
        let raw = std::fs::read(&command_plist).map_err(|e| AceError::Io(e.to_string()))?;
        let value: plist::Value =
            plist::from_bytes(&raw).map_err(|e| AceError::Message(e.to_string()))?;
        let dict = value
            .as_dictionary()
            .ok_or_else(|| AceError::msg("command.plist root must be a dict"))?;

        let name = dict
            .get("name")
            .or_else(|| dict.get("Name"))
            .and_then(|v| v.as_string())
            .unwrap_or("Unnamed Bundle")
            .to_string();

        let command = dict
            .get("command")
            .or_else(|| dict.get("Command"))
            .and_then(|v| v.as_string())
            .unwrap_or("")
            .to_string();

        let input = parse_input(
            dict.get("input")
                .or_else(|| dict.get("Input"))
                .and_then(|v| v.as_string())
                .unwrap_or("none"),
        );
        let output = parse_output(
            dict.get("output")
                .or_else(|| dict.get("Output"))
                .and_then(|v| v.as_string())
                .unwrap_or("discard"),
        );

        Ok(Self {
            id: Uuid::new_v4(),
            path: dir.to_path_buf(),
            manifest: BundleManifest {
                name,
                category: dict
                    .get("category")
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string()),
                trigger: dict
                    .get("trigger")
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string()),
                input,
                output,
                command,
                scope: dict
                    .get("scope")
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string()),
                key_equivalent: dict
                    .get("keyEquivalent")
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string()),
            },
        })
    }

    pub fn run(&self, input: &BundleInputData) -> AceResult<BundleResult> {
        let stdin_data = match self.manifest.input {
            BundleInput::None => String::new(),
            BundleInput::SelectedText => input.selected_text.clone().unwrap_or_default(),
            BundleInput::SelectedTableRows => input.table_tsv.clone().unwrap_or_default(),
            BundleInput::CurrentWord => input.current_word.clone().unwrap_or_default(),
            BundleInput::EntireContent => input.entire_content.clone().unwrap_or_default(),
        };

        #[cfg(windows)]
        let mut cmd = {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(&self.manifest.command);
            c
        };
        #[cfg(not(windows))]
        let mut cmd = {
            let mut c = Command::new("bash");
            c.arg("-lc").arg(&self.manifest.command);
            c
        };

        cmd.current_dir(&self.path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| AceError::Io(format!("bundle spawn: {e}")))?;
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(stdin_data.as_bytes());
        }
        let output = child
            .wait_with_output()
            .map_err(|e| AceError::Io(format!("bundle wait: {e}")))?;

        Ok(BundleResult {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
            output_kind: self.manifest.output,
        })
    }
}

fn parse_input(s: &str) -> BundleInput {
    match s.to_lowercase().as_str() {
        "selectedtext" | "selected_text" | "selection" => BundleInput::SelectedText,
        "selectedtablerows" | "selected_table_rows" | "tablerows" => BundleInput::SelectedTableRows,
        "currentword" | "current_word" => BundleInput::CurrentWord,
        "entirecontent" | "entire_content" => BundleInput::EntireContent,
        _ => BundleInput::None,
    }
}

fn parse_output(s: &str) -> BundleOutput {
    match s.to_lowercase().as_str() {
        "replaceselection" | "replace_selection" => BundleOutput::ReplaceSelection,
        "insertastext" | "insert_as_text" | "inserttext" => BundleOutput::InsertAsText,
        "showashtml" | "show_as_html" | "html" => BundleOutput::ShowAsHtml,
        "showastooltip" | "show_as_tooltip" | "tooltip" => BundleOutput::ShowAsTooltip,
        "showastext" | "show_as_text" | "text" => BundleOutput::ShowAsText,
        _ => BundleOutput::Discard,
    }
}

pub fn discover_bundles(dirs: &[PathBuf]) -> Vec<Bundle> {
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("saBundle")
                || path.join("command.plist").exists()
            {
                if let Ok(b) = Bundle::load(&path) {
                    out.push(b);
                }
            }
        }
    }
    out
}

/// Built-in bundle definitions shipped with Ace (mirror of Sequel Ace defaults).
pub fn builtin_bundle_specs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Format SQL", "sqlformat || cat"),
        ("Copy as JSON", "python3 -c \"import sys,json,csv; r=csv.DictReader(sys.stdin,delimiter='\\t'); print(json.dumps(list(r),indent=2))\""),
        ("Copy as Markdown", "awk 'BEGIN{FS=\"\\t\"} NR==1{print \"|\"$0\"|\"; gsub(/[^\\t]//,\"---\"); print \"|\"$0\"|\"} NR>1{print \"|\"$0\"|\"}'"),
        ("MD5 Hash", "md5 2>/dev/null || md5sum"),
        ("SHA1 Hash", "shasum -a 1"),
    ]
}