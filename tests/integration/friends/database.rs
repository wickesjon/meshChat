use meshchat_core::storage::{SqlDatabase, SqlRow, SqlValue, StorageError};
use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub fn hex(bytes: &[u8]) -> String {
    {
        use std::fmt::Write as _;
        let mut out = String::new();
        for b in bytes {
            write!(out, "{b:02x}").unwrap();
        }
        out
    }
}
pub fn unhex(raw: &str) -> Vec<u8> {
    raw.as_bytes()
        .chunks_exact(2)
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}
struct Worker {
    process: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
#[derive(Clone)]
pub struct Database {
    worker: Arc<Mutex<Worker>>,
    pub path: PathBuf,
}
impl Database {
    pub fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let folder = root.join(".work/friends-tests");
        std::fs::create_dir_all(&folder).unwrap();
        Self::reopen(folder.join(format!(
            "{}-{}-{}.sqlite",
            std::process::id(),
            module_path!().replace("::", "-"),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
    pub fn reopen(path: PathBuf) -> Self {
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/integration/friends/sqlite_worker.py");
        let mut child = Command::new(if cfg!(windows) { "python" } else { "python3" })
            .arg("-B")
            .arg(script)
            .arg(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            worker: Arc::new(Mutex::new(Worker {
                process: child,
                input,
                output,
            })),
            path,
        }
    }
    pub fn fail(&self, prefix: Option<&str>) {
        self.call('f', prefix.unwrap_or("-"), &[], 0).unwrap();
    }
    fn call(
        &self,
        mode: char,
        sql: &str,
        values: &[SqlValue],
        limit: u32,
    ) -> Result<Vec<SqlRow>, StorageError> {
        let mut w = self.worker.lock().unwrap();
        let values = values
            .iter()
            .map(|v| match v {
                SqlValue::Integer { value } => format!("i{value}"),
                SqlValue::Bytes { value } => format!("b{}", hex(value)),
                SqlValue::Text { value } => format!("t{}", hex(value.as_bytes())),
            })
            .collect::<Vec<_>>()
            .join(" ");
        writeln!(w.input, "{mode} {limit} {} {values}", hex(sql.as_bytes())).unwrap();
        w.input.flush().unwrap();
        let mut line = String::new();
        w.output.read_line(&mut line).unwrap();
        let Some(count) = line.strip_prefix("ok ") else {
            return Err(StorageError::Database);
        };
        let count: usize = count.trim().parse().unwrap();
        let mut rows = Vec::new();
        for _ in 0..count {
            line.clear();
            w.output.read_line(&mut line).unwrap();
            let cells = line
                .split_whitespace()
                .map(|v| match &v[..1] {
                    "i" => SqlValue::Integer {
                        value: v[1..].parse().unwrap(),
                    },
                    "b" => SqlValue::Bytes {
                        value: unhex(&v[1..]),
                    },
                    _ => SqlValue::Text {
                        value: String::from_utf8(unhex(&v[1..])).unwrap(),
                    },
                })
                .collect();
            rows.push(SqlRow { cells });
        }
        Ok(rows)
    }
}
impl SqlDatabase for Database {
    fn execute(&self, sql: String, values: Vec<SqlValue>) -> Result<(), StorageError> {
        self.call('x', &sql, &values, 0).map(|_| ())
    }
    fn query(
        &self,
        sql: String,
        values: Vec<SqlValue>,
        limit: u32,
    ) -> Result<Vec<SqlRow>, StorageError> {
        self.call('q', &sql, &values, limit)
    }
}
