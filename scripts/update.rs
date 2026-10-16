#!/usr/bin/env -S mise exec -- cargo -Zscript
---cargo
[package]
edition = "2024"

[dependencies]
anyhow = "1"
regex = "1"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
tempfile = "3"
yaml-edit = "0.3.2"
---

use std::{collections::BTreeMap, env, fs, path::Path, process::Command, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use yaml_edit::{Document, YamlFile, path::YamlPath};

const APPS: [&str; 3] = ["bot", "mc-link-server", "public-api"];
const COMPONENTS: [&str; 5] = ["bot", "mc-link-server", "public-api", "db-migrator", "chart"];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Artifact {
    component: String,
    repository: String,
    revision: String,
    digest: String,
    tag: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    release: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct State {
    deployed: BTreeMap<String, Artifact>,
    pending: BTreeMap<String, Artifact>,
}

fn repository(component: &str) -> Result<String> {
    ensure!(COMPONENTS.contains(&component), "未知の component: {component}");
    Ok(format!(
        "ghcr.io/anvilsaba/{}",
        if component == "chart" {
            "charts/platform"
        } else {
            component
        }
    ))
}

fn validate(artifact: &Artifact) -> Result<()> {
    ensure!(
        artifact.repository == repository(&artifact.component)?,
        "公開先が一致しません"
    );
    ensure!(
        Regex::new("^[a-f0-9]{40}$")?.is_match(&artifact.revision),
        "revision が不正です"
    );
    ensure!(
        Regex::new("^sha256:[a-f0-9]{64}$")?.is_match(&artifact.digest),
        "digest が不正です"
    );
    ensure!(
        Regex::new("^[a-zA-Z0-9_][a-zA-Z0-9_.-]{0,127}$")?.is_match(&artifact.tag),
        "tag が不正です"
    );
    Ok(())
}

fn run(command: &mut Command) -> Result<String> {
    let output = command.output().context("コマンドを起動できません")?;
    ensure!(
        output.status.success(),
        "コマンド失敗: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn git(source: &Path, args: &[&str]) -> Result<String> {
    run(Command::new("git").arg("-C").arg(source).args(args))
}

fn ancestor(source: &Path, before: &str, after: &str) -> Result<bool> {
    let pattern = Regex::new("^[a-f0-9]{40}$")?;
    for revision in [before, after] {
        ensure!(pattern.is_match(revision), "revision が不正です");
        git(source, &["cat-file", "-e", &format!("{revision}^{{commit}}")])?;
    }
    let status = Command::new("git")
        .arg("-C")
        .arg(source)
        .args(["merge-base", "--is-ancestor", before, after])
        .status()?;
    ensure!(matches!(status.code(), Some(0 | 1)), "Git 履歴を検証できません");
    Ok(status.success())
}

fn newer(source: &Path, current: Option<&Artifact>, candidate: &Artifact) -> Result<bool> {
    let Some(current) = current else { return Ok(true) };
    if current.revision == candidate.revision {
        ensure!(
            current.digest == candidate.digest,
            "同じ revision の digest が異なります"
        );
        return Ok(false);
    }
    if ancestor(source, &current.revision, &candidate.revision)? {
        return Ok(true);
    }
    ensure!(
        ancestor(source, &candidate.revision, &current.revision)?,
        "分岐した履歴は自動更新できません"
    );
    Ok(false)
}

fn compatible(source: &Path, app: &Artifact, migrator: &Artifact) -> Result<bool> {
    let file = match app.component.as_str() {
        "bot" | "public-api" => "main.rs",
        "mc-link-server" => "link_store.rs",
        _ => bail!("アプリではありません"),
    };
    let source_file = git(
        source,
        &["show", &format!("{}:apps/{}/src/{file}", app.revision, app.component)],
    )?;
    let pattern = Regex::new(r"(?s)const REQUIRED_MIGRATIONS: &\[i64\] = &\[(.*?)\];")?;
    let required = pattern
        .captures(&source_file)
        .context("アプリの必須 migration を検証できません")?;
    let paths = git(
        source,
        &["ls-tree", "-r", "--name-only", &app.revision, "--", "migrations"],
    )?;
    for version in required[1].split(',').map(str::trim).filter(|item| !item.is_empty()) {
        ensure!(
            version.bytes().all(|byte| byte.is_ascii_digit()),
            "必須 migration の定義を解釈できません"
        );
        let path = paths
            .lines()
            .find(|path| path.starts_with(&format!("migrations/{version}_")) && path.ends_with(".up.sql"))
            .context("必須 migration の SQL がありません")?;
        let app_blob = git(source, &["rev-parse", &format!("{}:{path}", app.revision)])?;
        let migration_blob = git(
            source,
            &["rev-parse", "--verify", &format!("{}:{path}", migrator.revision)],
        );
        if migration_blob.as_deref().ok() != Some(app_blob.as_str()) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn staging(current: &State, input: &Artifact, source: &Path) -> Result<State> {
    validate(input)?;
    let expected = if input.component == "chart" {
        format!("0.0.0-sha.{}", input.revision)
    } else {
        format!("sha-{}", input.revision)
    };
    ensure!(
        input.tag == expected && input.release.is_empty(),
        "main の通知ではありません"
    );
    ensure!(
        ancestor(source, &input.revision, &git(source, &["rev-parse", "HEAD"])?)?,
        "platform/main に含まれない revision です"
    );
    let mut state = current.clone();
    let latest = state
        .pending
        .get(&input.component)
        .or_else(|| state.deployed.get(&input.component));
    if !newer(source, latest, input)? {
        return Ok(state);
    }
    state.pending.insert(input.component.clone(), input.clone());
    if let Some(migrator) = state
        .pending
        .get("db-migrator")
        .or_else(|| state.deployed.get("db-migrator"))
        .cloned()
    {
        for name in APPS {
            if let Some(app) = state.deployed.get(name) {
                ensure!(
                    compatible(source, app, &migrator)?,
                    "Migrator が既存アプリの必須 migration を満たしません"
                );
            }
        }
        state.deployed.insert("db-migrator".into(), migrator.clone());
        state.pending.remove("db-migrator");
        for name in APPS {
            if let Some(app) = state.pending.get(name)
                && compatible(source, app, &migrator)?
            {
                state.deployed.insert(
                    name.into(),
                    state.pending.remove(name).context("保留アプリがありません")?,
                );
            }
        }
    }
    if let Some(chart) = state.pending.remove("chart") {
        state.deployed.insert("chart".into(), chart);
    }
    Ok(state)
}

fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

// Only fixed GHCR URLs receive registry credentials.
fn registry(input: &Artifact) -> Result<(Artifact, Option<Vec<u8>>)> {
    ensure!(input.repository == repository(&input.component)?, "公開先が不正です");
    let client = Client::builder().timeout(Duration::from_secs(60)).build()?;
    let repo = input
        .repository
        .strip_prefix("ghcr.io/")
        .context("GHCR ではありません")?;
    let mut auth = client
        .get("https://ghcr.io/token")
        .query(&[("service", "ghcr.io"), ("scope", &format!("repository:{repo}:pull"))]);
    if let Ok(token) = env::var("REGISTRY_TOKEN")
        && !token.is_empty()
    {
        auth = auth.basic_auth(
            env::var("REGISTRY_USER").unwrap_or_else(|_| "gitops".into()),
            Some(token),
        );
    }
    let token: Value = auth.send()?.error_for_status()?.json()?;
    let token = token["token"].as_str().context("GHCR token がありません")?;
    resolve_artifact(input, |path| {
        Ok(client.get(format!("https://ghcr.io/v2/{repo}/{path}")).bearer_auth(token)
        .header("Accept", "application/vnd.oci.image.index.v1+json,application/vnd.oci.image.manifest.v1+json,application/vnd.docker.distribution.manifest.list.v2+json,application/vnd.docker.distribution.manifest.v2+json")
        .send()?.error_for_status()?.bytes()?.to_vec())
    })
}

fn resolve_artifact(input: &Artifact, get: impl Fn(&str) -> Result<Vec<u8>>) -> Result<(Artifact, Option<Vec<u8>>)> {
    let bytes = get(&format!("manifests/{}", input.tag))?;
    let actual = hash(&bytes);
    ensure!(
        input.digest.is_empty() || actual == input.digest,
        "公開済み digest が一致しません"
    );
    let manifest: Value = serde_json::from_slice(&bytes)?;
    let verified = |path: &str, expected: &str| -> Result<Vec<u8>> {
        let bytes = get(path)?;
        ensure!(hash(&bytes) == expected, "OCI 内容と digest が一致しません");
        Ok(bytes)
    };
    let chart = if input.component == "chart" {
        ensure!(
            manifest["annotations"]["org.opencontainers.image.revision"].as_str() == Some(&input.revision),
            "Chart revision が一致しません"
        );
        let layer = manifest["layers"]
            .as_array()
            .context("Chart layer がありません")?
            .iter()
            .find(|layer| layer["mediaType"] == "application/vnd.cncf.helm.chart.content.v1.tar+gzip")
            .context("Helm Chart ではありません")?;
        let digest = layer["digest"].as_str().context("Chart layer digest がありません")?;
        Some(verified(&format!("blobs/{digest}"), digest)?)
    } else {
        let manifests = if let Some(index) = manifest["manifests"].as_array() {
            index
                .iter()
                .filter(|item| item["platform"]["os"] != "unknown")
                .map(|item| {
                    let digest = item["digest"].as_str().context("image digest がありません")?;
                    Ok(serde_json::from_slice::<Value>(&verified(
                        &format!("manifests/{digest}"),
                        digest,
                    )?)?)
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            vec![manifest]
        };
        ensure!(!manifests.is_empty(), "実行可能な image がありません");
        for image in manifests {
            let digest = image["config"]["digest"]
                .as_str()
                .context("image config がありません")?;
            let config: Value = serde_json::from_slice(&verified(&format!("blobs/{digest}"), digest)?)?;
            ensure!(
                config["config"]["Labels"]["org.opencontainers.image.revision"].as_str() == Some(&input.revision),
                "image revision が一致しません"
            );
        }
        None
    };
    let mut artifact = input.clone();
    artifact.digest = actual;
    validate(&artifact)?;
    Ok((artifact, chart))
}

fn document(path: &Path) -> Result<Document> {
    let file = YamlFile::from_path(path)?;
    ensure!(file.documents().count() == 1, "単一の YAML document が必要です");
    file.document().context("YAML document がありません")
}

fn image_key(component: &str) -> Result<&str> {
    Ok(match component {
        "bot" => "bot",
        "mc-link-server" => "mcLinkServer",
        "public-api" => "publicApi",
        "db-migrator" => "dbMigrator",
        _ => bail!("image component ではありません"),
    })
}

fn values_for(state: &State, values: &Document) -> Result<()> {
    for (name, artifact) in &state.deployed {
        if name == "chart" {
            continue;
        }
        let key = image_key(name)?;
        for (field, value) in [
            ("repository", &artifact.repository),
            ("tag", &artifact.tag),
            ("digest", &artifact.digest),
        ] {
            values.try_set_path(&format!("{key}.image.{field}"), value.as_str())?;
        }
    }
    if state.deployed.contains_key("db-migrator") {
        values.try_set_path("dbMigrator.enabled", true)?;
    }
    Ok(())
}

fn check_chart(
    artifact: &Artifact,
    values: &Document,
    environment: &Path,
    resolve: impl Fn(&Artifact) -> Result<(Artifact, Option<Vec<u8>>)>,
) -> Result<()> {
    let (_, chart) = resolve(artifact)?;
    let directory = tempfile::tempdir()?;
    let archive = directory.path().join("chart.tgz");
    let overrides = directory.path().join("values.yaml");
    fs::write(&archive, chart.context("Chart がありません")?)?;
    fs::write(&overrides, values.to_string())?;
    run(Command::new("helm")
        .arg("lint")
        .arg(&archive)
        .arg("-f")
        .arg(environment)
        .arg("-f")
        .arg(&overrides))?;
    let rendered = run(Command::new("helm")
        .args(["template", "platform"])
        .arg(&archive)
        .arg("-f")
        .arg(environment)
        .arg("-f")
        .arg(&overrides))?;
    for name in ["bot", "mcLinkServer", "publicApi", "dbMigrator"] {
        if let Ok(digest) = scalar(values, &format!("{name}.image.digest")) {
            let repo = scalar(values, &format!("{name}.image.repository"))?;
            ensure!(
                rendered.contains(&format!("{repo}@{digest}")),
                "Chart が image digest を反映しません"
            );
        }
    }
    Ok(())
}

fn scalar(document: &Document, path: &str) -> Result<String> {
    document
        .try_get_path(path)?
        .as_scalar()
        .map(|value| value.as_string())
        .context("YAML scalar がありません")
}

fn update_staging(
    payload: &[u8],
    directory: &Path,
    source: &Path,
    resolve: impl Fn(&Artifact) -> Result<(Artifact, Option<Vec<u8>>)>,
) -> Result<()> {
    let path = directory.join("artifacts.json");
    let current: State = serde_json::from_slice(&fs::read(&path)?)?;
    let input: Artifact = serde_json::from_slice(payload)?;
    validate(&input)?;
    let (input, _) = resolve(&input)?;
    let state = staging(&current, &input, source)?;
    if state.deployed != current.deployed {
        let values = document(&directory.join("values.yaml"))?;
        values_for(&state, &values)?;
        if COMPONENTS.iter().all(|name| state.deployed.contains_key(*name)) {
            check_chart(
                &state.deployed["chart"],
                &values,
                &directory.join("environment-values.yaml"),
                &resolve,
            )?;
        }
        let chart = document(&directory.join("chart.yaml"))?;
        if let Some(artifact) = state.deployed.get("chart") {
            chart.try_remove_path("spec.ref")?;
            chart.try_set_path("spec.ref.digest", artifact.digest.as_str())?;
        }
        fs::write(directory.join("values.yaml"), values.to_string())?;
        fs::write(directory.join("chart.yaml"), chart.to_string())?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&state)?))?;
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    ensure!(
        args.len() == 3 && args[0] == "staging",
        "使い方: update.rs staging <通知.json> <platform checkout>"
    );
    update_staging(
        &fs::read(&args[1])?,
        Path::new("apps/staging/platform"),
        Path::new(&args[2]),
        registry,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history() -> Result<(tempfile::TempDir, String, String)> {
        let repo = tempfile::tempdir()?;
        for args in [
            &["init"][..],
            &["config", "user.name", "test"],
            &["config", "user.email", "test@example.test"],
        ] {
            git(repo.path(), args)?;
        }
        fs::create_dir_all(repo.path().join("apps/bot/src"))?;
        fs::create_dir(repo.path().join("migrations"))?;
        fs::write(
            repo.path().join("apps/bot/src/main.rs"),
            "const REQUIRED_MIGRATIONS: &[i64] = &[20260926184758];",
        )?;
        fs::write(
            repo.path().join("migrations/20260926184758_base.up.sql"),
            "CREATE TABLE example (id bigint);",
        )?;
        git(repo.path(), &["add", "."])?;
        git(repo.path(), &["commit", "-m", "a"])?;
        let a = git(repo.path(), &["rev-parse", "HEAD"])?;
        git(repo.path(), &["commit", "--allow-empty", "-m", "b"])?;
        let b = git(repo.path(), &["rev-parse", "HEAD"])?;
        Ok((repo, a, b))
    }

    fn artifact(name: &str, revision: &str) -> Artifact {
        Artifact {
            component: name.into(),
            repository: repository(name).unwrap(),
            revision: revision.into(),
            digest: format!("sha256:{}", "a".repeat(64)),
            tag: if name == "chart" {
                format!("0.0.0-sha.{revision}")
            } else {
                format!("sha-{revision}")
            },
            release: String::new(),
        }
    }

    #[test]
    fn initial_notifications_write_available_references_without_waiting_for_other_apps() -> Result<()> {
        let (repo, a, b) = history()?;
        let directory = tempfile::tempdir()?;
        fs::write(
            directory.path().join("artifacts.json"),
            serde_json::to_vec(&State::default())?,
        )?;
        fs::write(directory.path().join("values.yaml"), "{}\n")?;
        fs::write(directory.path().join("chart.yaml"), "spec:\n  ref:\n    tag: 1.0.0\n")?;
        let notify = |input: Artifact| {
            update_staging(
                &serde_json::to_vec(&input)?,
                directory.path(),
                repo.path(),
                |artifact| Ok((artifact.clone(), None)),
            )
        };
        notify(artifact("bot", &b))?;
        assert_eq!(fs::read_to_string(directory.path().join("values.yaml"))?, "{}\n");
        notify(artifact("db-migrator", &a))?;
        notify(artifact("chart", &b))?;
        let values = document(&directory.path().join("values.yaml"))?;
        assert_eq!(scalar(&values, "bot.image.digest")?, artifact("bot", &b).digest);
        assert_eq!(scalar(&values, "dbMigrator.image.tag")?, format!("sha-{a}"));
        assert!(values.try_get_path("mcLinkServer").is_err());
        assert!(values.try_get_path("publicApi").is_err());
        let chart = document(&directory.path().join("chart.yaml"))?;
        assert_eq!(scalar(&chart, "spec.ref.digest")?, artifact("chart", &b).digest);
        assert!(chart.try_get_path("spec.ref.tag").is_err());
        let before = fs::read(directory.path().join("values.yaml"))?;
        notify(artifact("bot", &b))?;
        notify(artifact("bot", &a))?;
        assert_eq!(fs::read(directory.path().join("values.yaml"))?, before);
        assert!(update_staging(b"{}", directory.path(), repo.path(), |_| unreachable!()).is_err());
        assert_eq!(fs::read(directory.path().join("values.yaml"))?, before);
        Ok(())
    }

    #[test]
    fn notifications_wait_for_migrator_in_either_order_without_rolling_back() -> Result<()> {
        let (repo, a, b) = history()?;
        let app = artifact("bot", &b);
        let migrator = artifact("db-migrator", &b);
        let pending = staging(&State::default(), &app, repo.path())?;
        assert!(pending.deployed.is_empty());
        let state = staging(&pending, &migrator, repo.path())?;
        assert_eq!(state.deployed["bot"], app);
        assert_eq!(state.deployed.len(), 2);
        let reverse = staging(&staging(&State::default(), &migrator, repo.path())?, &app, repo.path())?;
        assert_eq!(state, reverse);
        let older_migrator = staging(&State::default(), &artifact("db-migrator", &a), repo.path())?;
        let app_only = staging(&older_migrator, &app, repo.path())?;
        assert_eq!(app_only.deployed["bot"], app);
        assert_eq!(app_only.deployed["db-migrator"].revision, a);
        assert_eq!(staging(&state, &app, repo.path())?, state);
        assert_eq!(staging(&state, &artifact("bot", &a), repo.path())?, state);
        let mut conflict = app.clone();
        conflict.digest = format!("sha256:{}", "b".repeat(64));
        assert!(staging(&state, &conflict, repo.path()).is_err());
        let chart = staging(&state, &artifact("chart", &b), repo.path())?;
        assert_eq!(chart.deployed["bot"], app);
        assert!(chart.deployed.contains_key("chart"));
        let mut invalid = app.clone();
        invalid.repository = "ghcr.io/other/bot".into();
        assert!(staging(&state, &invalid, repo.path()).is_err());
        fs::write(
            repo.path().join("apps/bot/src/main.rs"),
            "const REQUIRED_MIGRATIONS: &[i64] = &[20260926184758, 20260927120000];",
        )?;
        fs::write(
            repo.path().join("migrations/20260927120000_extra.up.sql"),
            "ALTER TABLE example ADD COLUMN label text;",
        )?;
        git(repo.path(), &["add", "."])?;
        git(repo.path(), &["commit", "-m", "required migration"])?;
        let c = git(repo.path(), &["rev-parse", "HEAD"])?;
        let waiting = staging(&app_only, &artifact("bot", &c), repo.path())?;
        assert_eq!(waiting.deployed["bot"].revision, b);
        assert_eq!(waiting.pending["bot"].revision, c);
        let ready = staging(&waiting, &artifact("db-migrator", &c), repo.path())?;
        assert_eq!(ready.deployed["bot"].revision, c);
        git(repo.path(), &["checkout", "--orphan", "diverged"])?;
        git(repo.path(), &["commit", "--allow-empty", "-m", "diverged"])?;
        assert!(
            staging(
                &state,
                &artifact("bot", &git(repo.path(), &["rev-parse", "HEAD"])?),
                repo.path()
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn forged_registry_digest_or_source_revision_is_rejected() -> Result<()> {
        let mut input = artifact("bot", &"a".repeat(40));
        let config = serde_json::to_vec(
            &serde_json::json!({"config":{"Labels":{"org.opencontainers.image.revision":input.revision}}}),
        )?;
        let manifest = serde_json::to_vec(&serde_json::json!({"config":{"digest":hash(&config)}}))?;
        input.digest = hash(&manifest);
        let get = |path: &str| {
            Ok(if path.starts_with("blobs/") {
                config.clone()
            } else {
                manifest.clone()
            })
        };
        assert_eq!(resolve_artifact(&input, get)?.0, input);
        input.revision = "b".repeat(40);
        assert!(resolve_artifact(&input, get).is_err());
        input.digest = format!("sha256:{}", "b".repeat(64));
        assert!(resolve_artifact(&input, get).is_err());
        Ok(())
    }
}
