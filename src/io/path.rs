use crate::dsp::Samples;
use color_eyre::eyre;
use hound::{SampleFormat, WavSpec, WavWriter};
use rodio::{Decoder, Source};
use std::cmp::Ordering;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Get path file name or descriptive error.
///
/// # Errors
///
/// Will return `Err` if `path` contains an unparseable name.
pub fn name(path: &Path) -> eyre::Result<&str> {
    path.file_name()
        .ok_or_else(|| eyre::eyre!("File path {:?} does not have a final component", path))?
        .to_str()
        .ok_or_else(|| eyre::eyre!("File name {:?} is not valid Unicode", path))
}

/// Read audio metdata and samples from a file.
///
/// # Errors
///
/// Will return `Err` if `path` cannot be opened or contains invalid audio data.
pub fn read_samples(path: &Path) -> eyre::Result<Samples> {
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let source = Decoder::new(reader)?;

    let channels = source.channels();
    let sample_rate = source.sample_rate();
    let samples: Vec<f32> = source.convert_samples().buffered().collect();
    Ok(Samples::new(channels, sample_rate, samples))
}

/// Read inodes from a directory and sort them with subdirectories first
///
/// # Errors
///
/// Will return `Err` if `directory` does not exist or contains files whose metadata is unparseable.
pub fn sorted_names(directory: &Path) -> eyre::Result<Vec<(String, bool)>> {
    let mut files: Vec<(String, bool)> = vec![];

    for inode in directory.read_dir()? {
        let inode = inode?;
        files.push((
            name(&inode.path())?.to_string(),
            inode.file_type()?.is_dir(),
        ));
    }

    files.sort_by(|left, right| {
        if left.1 && !right.1 {
            Ordering::Less
        } else if !left.1 && right.1 {
            Ordering::Greater
        } else {
            left.0.cmp(&right.0)
        }
    });
    Ok(files)
}

/// Write audio metdata and samples to a file.
///
/// # Errors
///
/// Will return `Err` if `path` is unwritable.
pub fn write_samples(path: &Path, samples: &Samples) -> eyre::Result<()> {
    let spec = WavSpec {
        channels: samples.channels,
        sample_rate: samples.sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let mut writer = WavWriter::create(path, spec)?;

    for sample in &samples.data {
        writer.write_sample(*sample)?;
    }

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::util;
    use approx::assert_abs_diff_eq;
    use std::fs;

    #[test]
    fn sort_folders_before_files() {
        let folder = tempfile::tempdir().unwrap().path().to_owned();
        fs::create_dir(&folder).unwrap();

        File::create(folder.join("a")).unwrap();
        fs::create_dir(folder.join("c")).unwrap();
        fs::create_dir(folder.join("b")).unwrap();
        File::create(folder.join("d")).unwrap();

        let expected = vec![
            (String::from("b"), true),
            (String::from("c"), true),
            (String::from("a"), false),
            (String::from("d"), false),
        ];
        let actual = sorted_names(&folder).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn write_and_read() {
        let expected = Samples::new(2, 32, vec![0.0f32, -0.25f32, 0.25f32, 1.0f32]);
        let path = util::test::temp_wave_file(&expected).unwrap();

        let actual = read_samples(&path).unwrap();
        assert_abs_diff_eq!(actual, expected, epsilon = 0.0001);
    }
}
