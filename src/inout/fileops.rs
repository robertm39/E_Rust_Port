use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::output::{out_close, out_open};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

#[derive(Debug)]
pub enum InputSource {
    Stdin,
    File(File),
}

impl InputSource {
    #[must_use]
    pub const fn is_stdin(&self) -> bool {
        matches!(self, Self::Stdin)
    }
}

impl Read for InputSource {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Stdin => io::stdin().lock().read(buffer),
            Self::File(file) => file.read(buffer),
        }
    }
}

fn io_diagnostic(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::FILE_ERROR, message)
}

fn maybe_fail<T>(fail: bool, diagnostic: Diagnostic) -> Result<Option<T>, Diagnostic> {
    if fail {
        Err(diagnostic)
    } else {
        Ok(None)
    }
}

pub fn input_open(name: Option<&Path>, fail: bool) -> Result<Option<InputSource>, Diagnostic> {
    let Some(path) = name else {
        return Ok(Some(InputSource::Stdin));
    };
    if path == Path::new("-") {
        return Ok(Some(InputSource::Stdin));
    }

    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => {
            return maybe_fail(
                fail,
                io_diagnostic(format!("{} it is not a regular file", path.display())),
            );
        }
        Err(error) => {
            return maybe_fail(
                fail,
                io_diagnostic(format!("Cannot stat file {}: {error}", path.display())),
            );
        }
    }

    File::open(path)
        .map(|file| Some(InputSource::File(file)))
        .map_err(|error| {
            io_diagnostic(format!(
                "Cannot open file {} for reading: {error}",
                path.display()
            ))
        })
        .or_else(|diagnostic| maybe_fail(fail, diagnostic))
}

fn input_open_required(name: Option<&Path>) -> Result<InputSource, Diagnostic> {
    input_open(name, true)?.ok_or_else(|| io_diagnostic("Cannot open input for reading"))
}

pub fn input_close(input: InputSource) -> Result<(), Diagnostic> {
    match input {
        InputSource::Stdin => Ok(()),
        InputSource::File(file) => {
            drop(file);
            Ok(())
        }
    }
}

pub fn file_load(name: &Path, dest: &mut DynamicString) -> Result<usize, Diagnostic> {
    let mut input = input_open_required(Some(name))?;
    let mut count = 0_usize;
    let mut buffer = [0_u8; 8192];

    loop {
        let read = input.read(&mut buffer).map_err(|error| {
            io_diagnostic(format!("Cannot read file {}: {error}", name.display()))
        })?;
        if read == 0 {
            break;
        }
        count = count.checked_add(read).ok_or_else(|| {
            io_diagnostic(format!("File {} is too large to load", name.display()))
        })?;
        dest.append_buffer(&buffer[..read]);
    }

    input_close(input)?;
    Ok(count)
}

pub fn concat_files<S>(target: &Path, sources: &[S]) -> Result<usize, Diagnostic>
where
    S: AsRef<Path>,
{
    let mut output = out_open(Some(target))?;
    let mut count = 0_usize;

    for source in sources {
        let path = source.as_ref();
        let mut input = input_open_required(Some(path))?;
        io::copy(&mut input, &mut output).map_err(|error| {
            io_diagnostic(format!(
                "Cannot copy file {} to {}: {error}",
                path.display(),
                target.display()
            ))
        })?;
        input_close(input)?;
        count = count
            .checked_add(1)
            .ok_or_else(|| io_diagnostic("Too many source files to concatenate"))?;
    }

    out_close(output)?;
    Ok(count)
}

pub fn copy_file(target: &Path, source: &Path) -> Result<usize, Diagnostic> {
    concat_files(target, &[source])
}

pub fn file_remove(name: &Path) -> Result<(), Diagnostic> {
    fs::remove_file(name)
        .map_err(|error| io_diagnostic(format!("Cannot remove file {}: {error}", name.display())))
}

pub fn file_print(output: &mut impl Write, name: &Path) -> Result<(), Diagnostic> {
    let mut input = input_open_required(Some(name))?;
    io::copy(&mut input, output)
        .map_err(|error| io_diagnostic(format!("Cannot print file {}: {error}", name.display())))?;
    input_close(input)?;
    Ok(())
}

#[must_use]
pub fn file_name_is_absolute(name: &str) -> bool {
    name.as_bytes().first() == Some(&b'/')
}

#[must_use]
pub fn file_name_dir_name(name: &str) -> String {
    let end = name.rfind('/').map_or(0, |index| index + 1);
    name[..end].to_owned()
}

#[must_use]
pub fn file_find_base_name(name: &str) -> &str {
    let start = name.rfind('/').map_or(0, |index| index + 1);
    &name[start..]
}

#[must_use]
pub fn file_name_base_name(name: &str) -> String {
    file_find_base_name(name).to_owned()
}

#[must_use]
pub fn file_name_strip(name: &str) -> String {
    let base = file_find_base_name(name);
    let mut strip_len = 0_usize;
    let mut full_len = 0_usize;

    for (index, byte) in base.bytes().enumerate() {
        full_len = index + 1;
        if byte == b'.' {
            strip_len = index;
        }
    }
    if strip_len == 0 {
        strip_len = full_len;
    }

    base[..strip_len].to_owned()
}

#[must_use]
pub fn file_exists(name: &Path) -> bool {
    File::open(name).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{
        concat_files, copy_file, file_exists, file_find_base_name, file_load, file_name_base_name,
        file_name_dir_name, file_name_is_absolute, file_name_strip, file_print, file_remove,
        input_close, input_open, InputSource,
    };
    use crate::basics::dstrings::DynamicString;
    use crate::basics::error::ErrorCode;
    use std::path::{Path, PathBuf};

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("fileops-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    #[test]
    fn input_open_matches_stdin_fail_and_regular_file_cases() {
        let missing = temp_path("missing");
        remove_if_present(&missing);

        assert!(input_open(Some(&missing), false).unwrap().is_none());
        let error = input_open(Some(&missing), true).unwrap_err();
        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error.message().contains("Cannot stat file"));

        assert!(input_open(None, true).unwrap().unwrap().is_stdin());
        assert!(input_open(Some(Path::new("-")), true)
            .unwrap()
            .unwrap()
            .is_stdin());

        let directory_error = input_open(Some(Path::new("target")), true).unwrap_err();
        assert_eq!(directory_error.code(), ErrorCode::FILE_ERROR);
        assert!(directory_error.message().contains("not a regular file"));
    }

    #[test]
    fn file_load_appends_bytes_and_counts_them() {
        let path = temp_path("load");
        remove_if_present(&path);
        std::fs::write(&path, b"a\n\xff").unwrap();

        let mut dest = DynamicString::new();
        dest.append_str("pre");
        assert_eq!(file_load(&path, &mut dest).unwrap(), 3);
        assert_eq!(dest.view_bytes(), b"prea\n\xff");

        file_remove(&path).unwrap();
    }

    #[test]
    fn concat_and_copy_files_preserve_source_order() {
        let source_a = temp_path("source-a");
        let source_b = temp_path("source-b");
        let target = temp_path("target");
        let copy = temp_path("copy");
        for path in [&source_a, &source_b, &target, &copy] {
            remove_if_present(path);
        }
        std::fs::write(&source_a, b"ab").unwrap();
        std::fs::write(&source_b, b"cd").unwrap();

        assert_eq!(
            concat_files(&target, &[source_a.as_path(), source_b.as_path()]).unwrap(),
            2
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"abcd");

        assert_eq!(copy_file(&copy, &target).unwrap(), 1);
        assert_eq!(std::fs::read(&copy).unwrap(), b"abcd");

        for path in [&source_a, &source_b, &target, &copy] {
            file_remove(path).unwrap();
        }
    }

    #[test]
    fn file_print_exists_and_remove_match_c_helpers() {
        let path = temp_path("print");
        remove_if_present(&path);
        std::fs::write(&path, b"payload").unwrap();

        assert!(file_exists(&path));
        let mut output = Vec::new();
        file_print(&mut output, &path).unwrap();
        assert_eq!(output, b"payload");

        file_remove(&path).unwrap();
        assert!(!file_exists(&path));

        let error = file_remove(&path).unwrap_err();
        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
    }

    #[test]
    fn unix_style_name_helpers_preserve_c_quirks() {
        assert!(file_name_is_absolute("/tmp/e.p"));
        assert!(!file_name_is_absolute(""));
        assert!(!file_name_is_absolute("C:/tmp/e.p"));

        assert_eq!(file_name_dir_name("dir/sub/file.p"), "dir/sub/");
        assert_eq!(file_name_dir_name("file.p"), "");
        assert_eq!(file_name_dir_name("/"), "/");

        assert_eq!(file_find_base_name("dir/sub/file.p"), "file.p");
        assert_eq!(file_find_base_name("dir/sub/"), "");
        assert_eq!(file_name_base_name("dir/sub/file.p"), "file.p");
        assert_eq!(file_name_base_name("dir/sub/"), "");

        assert_eq!(file_name_strip("dir/a.b.c"), "a.b");
        assert_eq!(file_name_strip("dir/name."), "name");
        assert_eq!(file_name_strip("plain"), "plain");
        assert_eq!(file_name_strip(".hidden"), ".hidden");
        assert_eq!(file_name_strip("dir/"), "");
    }

    #[test]
    fn input_source_reads_regular_files() {
        let path = temp_path("input-read");
        remove_if_present(&path);
        std::fs::write(&path, b"readable").unwrap();

        let mut source = input_open(Some(&path), true).unwrap().unwrap();
        assert!(matches!(source, InputSource::File(_)));
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut source, &mut bytes).unwrap();
        assert_eq!(bytes, b"readable");
        input_close(source).unwrap();

        file_remove(&path).unwrap();
    }

    #[test]
    fn input_close_skips_stdin_and_reports_successful_file_close() {
        assert!(input_close(InputSource::Stdin).is_ok());

        let path = temp_path("input-close");
        remove_if_present(&path);
        std::fs::write(&path, b"close").unwrap();
        let source = input_open(Some(&path), true).unwrap().unwrap();

        input_close(source).unwrap();
        file_remove(&path).unwrap();
    }
}
