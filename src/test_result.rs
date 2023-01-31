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
    pub responses: Vec<ResponseInfo>,
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
        Ok(Self::rkyv_files(output_path)?
            .map(Self::unarchive_file)
            .filter_map(|res| res.ok()))
    }

    pub fn load_filtered(
        output_path: &str,
        names: Option<&[String]>,
    ) -> Result<impl Iterator<Item = TestResult>> {
        Ok(Self::rkyv_files(output_path)?
            .filter(|file| match names {
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

    fn rkyv_files(directory: &str) -> Result<impl Iterator<Item = DirEntry>> {
        Ok(fs::read_dir(directory)?
            .into_iter()
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
