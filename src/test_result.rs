use crate::ResponseInfo;
use anyhow::Result;
use rkyv::{check_archived_root, to_bytes, Archive, Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs::{self, DirEntry, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};
use tokio::time::Duration;

#[derive(Debug, Serialize, Deserialize, Archive)]
#[archive(check_bytes)]
pub struct TestResult {
    pub name: String,
    total_time: Duration,
    request_time: Duration,
    pub responses: Vec<ResponseInfo>,
    success_count: usize,
    failure_count: usize,
}

impl TestResult {
    pub fn new(responses: Vec<ResponseInfo>, name: String, total_time: Duration) -> Self {
        let success_count = responses.iter().filter(|r| r.status.is_success()).count();
        let failure_count = responses.iter().filter(|r| !r.status.is_success()).count();
        let request_time = responses.iter().map(|r| r.time).sum();
        Self {
            name,
            total_time,
            responses,
            success_count,
            failure_count,
            request_time,
        }
    }

    pub fn success_responses(&self) -> impl Iterator<Item = &ResponseInfo> {
        self.responses.iter().filter(|r| r.status.is_success())
    }
    pub fn failure_responses(&self) -> impl Iterator<Item = &ResponseInfo> {
        self.responses.iter().filter(|r| !r.status.is_success())
    }

    fn success_total_time(&self) -> Duration {
        self.success_responses().map(|r| r.time).sum()
    }

    pub fn failure_total_time(&self) -> Duration {
        self.failure_responses().map(|r| r.time).sum()
    }

    pub fn avg_success(&self) -> Option<Duration> {
        self.success_total_time()
            .checked_div(self.success_count as u32)
    }

    pub fn avg_failure(&self) -> Option<Duration> {
        self.failure_total_time()
            .checked_div(self.failure_count as u32)
    }

    pub fn report(&self) -> String {
        format!(
            "{}:
    time: {:?} (~{} rps)
    success: {} ({:?} avg)
    failure: {} ({:?} avg)
        ",
            self.name,
            self.total_time,
            ((self.success_count + self.failure_count) as f64 / self.total_time.as_secs_f64())
                as u64,
            self.success_count,
            self.avg_success().unwrap_or(Duration::from_secs(0)),
            self.failure_count,
            self.avg_failure().unwrap_or(Duration::from_secs(0))
        )
    }

    fn file_path<P: AsRef<Path>>(&self, folder: P) -> PathBuf {
        folder.as_ref().join(format!("{}.rkyv", self.name))
    }

    pub fn save<P: AsRef<Path>>(&self, folder: P) -> Result<()> {
        let file_path = self.file_path(&folder);
        let _ = fs::create_dir_all(&folder);
        let mut writer = BufWriter::new(File::create(file_path)?);
        let bytes = to_bytes::<Self, 1024>(self)?;
        Ok(writer.write_all(&bytes)?)
    }

    pub fn load_filtered<P: AsRef<Path>>(
        data_dir: P,
        names: Option<Vec<String>>,
    ) -> Result<impl Iterator<Item = TestResult>> {
        Ok(Self::rkyv_files(data_dir)?
            .filter(|file| match &names {
                Some(names) => file
                    .path()
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .map(|v| names.iter().any(|n| n == v))
                    .unwrap_or(false),
                None => true,
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(Self::unarchive_file)
            .filter_map(|res| res.ok()))
    }

    fn rkyv_files<P: AsRef<Path>>(directory: P) -> Result<impl Iterator<Item = DirEntry>> {
        Ok(fs::read_dir(directory)?
            .filter_map(|f| f.ok())
            .filter(|file| file.path().extension().and_then(OsStr::to_str) == Some("rkyv")))
    }

    fn unarchive_file(dir: DirEntry) -> Result<TestResult> {
        let file = fs::read(dir.path())?;
        let archived = check_archived_root::<TestResult>(&file).unwrap();

        let result: TestResult = archived.deserialize(&mut rkyv::Infallible)?;

        Ok(result)
    }
}
