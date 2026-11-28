use std::collections::BTreeSet;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const EDGES: [(&str, &str); 9] = [
    ("billing", "kernel"),
    ("identity", "kernel"),
    ("router", "billing"),
    ("router", "kernel"),
    ("runtime", "billing"),
    ("runtime", "identity"),
    ("runtime", "kernel"),
    ("runtime", "router"),
    ("supply", "kernel"),
];

const SCRIPT: &str = r#"
import json, subprocess, sys
root, locked, cargo = sys.argv[1], sys.argv[2], sys.argv[3]
cmd = [cargo, "metadata", "--format-version", "1", "--no-deps", "--offline", "--manifest-path", root + "/Cargo.toml"]
if locked == "1":
    cmd.append("--locked")
proc = subprocess.run(cmd, capture_output=True, text=True)
if proc.returncode != 0:
    sys.stderr.write(proc.stderr)
    raise SystemExit(proc.returncode)
meta = json.loads(proc.stdout)
members = set(meta["workspace_members"])
by_dir = {}
for pkg in meta["packages"]:
    if pkg["id"] not in members:
        continue
    manifest = pkg["manifest_path"]
    by_dir[manifest[: manifest.rfind("/")]] = pkg["name"]
    sys.stdout.write("name\t%s\n" % pkg["name"])
for pkg in meta["packages"]:
    if pkg["id"] not in members:
        continue
    for dep in pkg["dependencies"]:
        path = dep.get("path")
        target = by_dir.get(path) if path else None
        if target:
            sys.stdout.write("edge\t%s\t%s\n" % (pkg["name"], target))
"#;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Fault {
    UnknownCrate { name: String },
    Reverse { from: String, to: String },
    SameLayer { from: String, to: String },
    Read { path: String, detail: String },
    Parse { path: String, detail: String },
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCrate { name } => write!(formatter, "未知 crate {name}"),
            Self::Reverse { from, to } => write!(formatter, "{from} 反向依赖 {to}"),
            Self::SameLayer { from, to } => write!(formatter, "{from} 与 {to} 同层互引"),
            Self::Read { path, detail } => write!(formatter, "读取 {path} 失败：{detail}"),
            Self::Parse { path, detail } => write!(formatter, "解析 {path} 失败：{detail}"),
        }
    }
}

struct Graph {
    names: BTreeSet<String>,
    edges: BTreeSet<(String, String)>,
}

struct Loaded {
    names: BTreeSet<String>,
    edges: BTreeSet<(String, String)>,
    faults: Vec<Fault>,
}

fn rank(name: &str) -> Option<u8> {
    match name {
        "kernel" => Some(0),
        "identity" | "billing" | "supply" => Some(1),
        "router" => Some(2),
        "runtime" => Some(3),
        _ => None,
    }
}

fn classify(from: &str, to: &str) -> Option<Fault> {
    let (Some(source_rank), Some(dependency_rank)) = (rank(from), rank(to)) else {
        return None;
    };
    if source_rank > dependency_rank {
        return None;
    }
    if source_rank == dependency_rank {
        return Some(Fault::SameLayer {
            from: from.to_owned(),
            to: to.to_owned(),
        });
    }
    Some(Fault::Reverse {
        from: from.to_owned(),
        to: to.to_owned(),
    })
}

fn direction_faults(names: &BTreeSet<String>, edges: &BTreeSet<(String, String)>) -> Vec<Fault> {
    let mut faults = BTreeSet::new();
    for name in names {
        if rank(name).is_none() {
            faults.insert(Fault::UnknownCrate { name: name.clone() });
        }
    }
    for (from, to) in edges {
        if let Some(fault) = classify(from, to) {
            faults.insert(fault);
        }
    }
    faults.into_iter().collect()
}

fn parse_report(text: &str) -> Result<Graph, String> {
    let mut names = BTreeSet::new();
    let mut edges = BTreeSet::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split('\t');
        match parts.next() {
            Some("name") => {
                let name = parts.next().ok_or_else(|| "缺少包名".to_owned())?;
                names.insert(name.to_owned());
            }
            Some("edge") => {
                let from = parts.next().ok_or_else(|| "缺少依赖边".to_owned())?;
                let to = parts.next().ok_or_else(|| "缺少依赖边".to_owned())?;
                edges.insert((from.to_owned(), to.to_owned()));
            }
            Some(_) | None => return Err(format!("无法识别的行 {line}")),
        }
    }
    Ok(Graph { names, edges })
}

fn read_graph(root: &Path, locked: bool) -> Result<Loaded, Fault> {
    let root = root.canonicalize().map_err(|error| Fault::Read {
        path: root.display().to_string(),
        detail: error.to_string(),
    })?;
    let cargo = env::var_os("CARGO").map_or_else(|| PathBuf::from("cargo"), PathBuf::from);
    let target = unique_dir();
    let output = Command::new("python3")
        .arg("-c")
        .arg(SCRIPT)
        .arg(&root)
        .arg(if locked { "1" } else { "0" })
        .arg(&cargo)
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .map_err(|error| Fault::Read {
            path: root.display().to_string(),
            detail: error.to_string(),
        })?;
    let _removed = fs::remove_dir_all(&target).is_err();
    if !output.status.success() {
        return Err(Fault::Parse {
            path: root.display().to_string(),
            detail: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let text = String::from_utf8(output.stdout).map_err(|error| Fault::Parse {
        path: root.display().to_string(),
        detail: error.to_string(),
    })?;
    let graph = parse_report(&text).map_err(|detail| Fault::Parse {
        path: root.display().to_string(),
        detail,
    })?;
    let faults = direction_faults(&graph.names, &graph.edges);
    Ok(Loaded {
        names: graph.names,
        edges: graph.edges,
        faults,
    })
}

fn faults_in(root: &Path) -> Vec<Fault> {
    match read_graph(root, false) {
        Ok(loaded) => loaded.faults,
        Err(fault) => vec![fault],
    }
}

fn unique_dir() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    env::temp_dir().join(format!("next-direction-{nanos}-{id}"))
}

struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let dir = unique_dir();
        match fs::create_dir_all(&dir) {
            Ok(()) => Self { dir },
            Err(error) => panic!("创建临时目录失败：{error}"),
        }
    }

    fn write(&self, rel: &str, body: &str) {
        let path = self.dir.join(rel);
        if let Some(parent) = path.parent() {
            match fs::create_dir_all(parent) {
                Ok(()) => {}
                Err(error) => panic!("创建 {rel} 失败：{error}"),
            }
        }
        match fs::write(&path, body) {
            Ok(()) => {}
            Err(error) => panic!("写入 {rel} 失败：{error}"),
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = fs::remove_dir_all(&self.dir).is_err();
    }
}

fn package(name: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.5.0\"\nedition = \"2021\"\n{extra}")
}

fn add_crate(scratch: &Scratch, rel: &str, name: &str, extra: &str) {
    scratch.write(&format!("{rel}/Cargo.toml"), &package(name, extra));
    scratch.write(&format!("{rel}/src/lib.rs"), "\n");
}

fn pair(deps: &str) -> Scratch {
    let scratch = Scratch::new();
    scratch.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/kernel\", \"crates/runtime\"]\n",
    );
    add_crate(&scratch, "crates/runtime", "runtime", "");
    add_crate(&scratch, "crates/kernel", "kernel", deps);
    scratch
}

fn reverse_kernel_runtime() -> Vec<Fault> {
    vec![Fault::Reverse {
        from: "kernel".to_owned(),
        to: "runtime".to_owned(),
    }]
}

#[test]
fn workspace_follows_direction() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let loaded = match read_graph(&root, true) {
        Ok(value) => value,
        Err(fault) => panic!("{fault}"),
    };
    for name in [
        "billing", "identity", "kernel", "router", "runtime", "supply",
    ] {
        assert!(loaded.names.contains(name), "缺少 {name}");
    }
    let expected = EDGES
        .into_iter()
        .map(|(from, to)| (from.to_owned(), to.to_owned()))
        .collect::<BTreeSet<_>>();
    assert_eq!(loaded.edges, expected);
    assert_eq!(loaded.faults, Vec::new());
}

#[test]
fn reverse_edges_fail() {
    let expected = reverse_kernel_runtime();
    let inline = "runtime = { path = \"../runtime\" }\n";
    for header in [
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "target.'cfg(unix)'.dependencies",
        "target.'cfg(windows)'.dependencies",
    ] {
        let scratch = pair(&format!("[{header}]\n{inline}"));
        assert_eq!(faults_in(&scratch.dir), expected, "{header}");
    }
    let scratch = pair("[dependencies.runtime]\npath = \"../runtime\"\n");
    assert_eq!(faults_in(&scratch.dir), expected);
    let scratch = pair("[dependencies]\nup = { path = \"../runtime\", package = \"runtime\" }\n");
    assert_eq!(faults_in(&scratch.dir), expected);
}

#[test]
fn workspace_dependency_reverse_fails() {
    let scratch = Scratch::new();
    scratch.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/kernel\", \"crates/runtime\"]\n\n[workspace.dependencies]\nruntime = { path = \"crates/runtime\", version = \"=0.5.0\" }\n",
    );
    add_crate(&scratch, "crates/runtime", "runtime", "");
    add_crate(
        &scratch,
        "crates/kernel",
        "kernel",
        "[dependencies]\nruntime = { workspace = true }\n",
    );
    assert_eq!(faults_in(&scratch.dir), reverse_kernel_runtime());
}

#[test]
fn same_layer_dependency_fails() {
    let scratch = Scratch::new();
    scratch.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/identity\", \"crates/billing\"]\n",
    );
    add_crate(&scratch, "crates/billing", "billing", "");
    add_crate(
        &scratch,
        "crates/identity",
        "identity",
        "[dependencies]\nbilling = { path = \"../billing\" }\n",
    );
    assert_eq!(
        faults_in(&scratch.dir),
        vec![Fault::SameLayer {
            from: "identity".to_owned(),
            to: "billing".to_owned(),
        }]
    );
}

#[test]
fn unknown_crate_fails() {
    let scratch = Scratch::new();
    scratch.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/extra\"]\n",
    );
    add_crate(&scratch, "crates/extra", "extra", "");
    assert_eq!(
        faults_in(&scratch.dir),
        vec![Fault::UnknownCrate {
            name: "extra".to_owned(),
        }]
    );
}
