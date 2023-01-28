use crate::ResponseInfo;
use anyhow::Result;
use bytecheck::CheckBytes;
use rkyv::{check_archived_root, to_bytes, Archive, Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs::{self, DirEntry, File},
    io::{BufWriter, Write},
    path::PathBuf,
};
use tokio::time::Duration;

#[derive(Debug, Serialize, Deserialize, Archive)]
#[archive_attr(derive(CheckBytes, Debug))]
pub struct TestResult {
    pub name: String,
    total_time: Duration,
    responses: Vec<ResponseInfo>,
    success_count: usize,
    failure_count: usize,
}

impl TestResult {
    pub fn new(responses: Vec<ResponseInfo>, name: String, total_time: Duration) -> Self {
        let success_count = responses.iter().filter(|r| r.status.is_success()).count();
        let failure_count = responses.iter().filter(|r| !r.status.is_success()).count();
        Self {
            name,
            total_time,
            responses,
            success_count,
            failure_count,
        }
    }

    pub fn success_responses<'a>(&'a self) -> impl Iterator<Item = &'a ResponseInfo> {
        self.responses.iter().filter(|r| r.status.is_success())
    }
    pub fn failure_responses<'a>(&'a self) -> impl Iterator<Item = &'a ResponseInfo> {
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
            (self.success_count + self.failure_count) / self.total_time.as_secs() as usize,
            self.success_count,
            self.avg_success().unwrap_or(Duration::from_secs(0)),
            self.failure_count,
            self.avg_failure().unwrap_or(Duration::from_secs(0))
        )
    }

    fn file_path(&self, folder: &str) -> PathBuf {
        let filename = format!("{}.rkyv", self.name);
        PathBuf::from(folder).join(filename)
    }

    pub fn save(&self, folder: &str) -> Result<()> {
        let file_path = self.file_path(folder);
        let _ = fs::create_dir_all(folder);
        let mut writer = BufWriter::new(File::create(file_path)?);
        let bytes = to_bytes::<Self, 1024>(self)?;
        Ok(writer.write_all(&bytes)?)
    }

    pub fn load_data(output_path: &str) -> Result<impl Iterator<Item = TestResult>> {
        Ok(fs::read_dir(output_path)?
            .into_iter()
            .filter_map(|f| f.ok())
            .filter(|file| file.path().extension().and_then(OsStr::to_str) == Some("rkyv"))
            .map(|file| Self::unarchive_file(file))
            .filter_map(|res| res.ok()))
    }

    fn unarchive_file(dir: DirEntry) -> Result<TestResult> {
        let file = fs::read(dir.path())?;
        let archived = check_archived_root::<TestResult>(&file).unwrap();

        let result: TestResult = archived.deserialize(&mut rkyv::Infallible)?;

        Ok(result)
    }
}
