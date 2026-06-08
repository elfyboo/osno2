use std::path::PathBuf;

pub struct FsEntry {
    pub path: PathBuf,
    pub name: String,
    pub ext: String,
    pub size_bytes: u64,
    pub is_dir: bool,
    pub kind: FsEntryKind,
}

pub enum FsEntryKind {
    Directory,
    AudioFile,
    File,
}
